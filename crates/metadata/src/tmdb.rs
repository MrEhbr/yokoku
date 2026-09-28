use async_trait::async_trait;
use serde::de::DeserializeOwned;
use yokoku_domain::{
    Artwork, EpisodeMetadata, ExternalId, Live, MediaKind, MovieMetadata, SeasonMetadata, SeriesMetadata,
};
use yokoku_library::ports::{MetadataError, MetadataProvider, SearchResult};

use crate::{
    MetadataSettings,
    http::{self, Http, invalid},
    tmdb_wire::{self, MovieDetails, SearchItem, SearchPage, SeasonDetails, TvDetails},
};

/// TMDB's limit on `append_to_response` entries per request.
const MAX_APPENDED_SEASONS: usize = 20;

/// TMDB API v3 with a read access token.
pub struct TmdbClient {
    http: Http,
    settings: Live<MetadataSettings>,
}

impl TmdbClient {
    pub fn new(settings: Live<MetadataSettings>) -> Self {
        Self { http: Http::new("TMDB"), settings }
    }

    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
        source: Option<ExternalId>,
    ) -> Result<T, MetadataError> {
        let settings = self.settings.current();
        let token = settings.tmdb.token.as_ref().ok_or_else(|| MetadataError::Refused("no TMDB token".into()))?;
        let request = self
            .http
            .get(format!("{}/{endpoint}", settings.tmdb.url))
            .bearer_auth(token.expose())
            .query(&[("language", settings.language.as_str())])
            .query(query);
        http::json(self.http.send(request, source).await?).await
    }
}

#[async_trait]
impl MetadataProvider for TmdbClient {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MetadataError> {
        let page: SearchPage = self.get("search/multi", &[("query", query), ("include_adult", "false")], None).await?;

        Ok(page
            .results
            .into_iter()
            .filter_map(|item| match item {
                SearchItem::Movie(movie) => Some(SearchResult {
                    kind: MediaKind::Movie,
                    source: ExternalId::Tmdb(movie.id),
                    year: http::year(movie.release_date.as_deref()),
                    title: movie.title,
                    original_title: movie.original_title,
                    poster_path: movie.poster_path,
                }),
                SearchItem::Tv(tv) => Some(SearchResult {
                    kind: MediaKind::Series,
                    source: ExternalId::Tmdb(tv.id),
                    year: http::year(tv.first_air_date.as_deref()),
                    title: tv.name,
                    original_title: tv.original_name,
                    poster_path: tv.poster_path,
                }),
                SearchItem::Other => None,
            })
            .collect())
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        let id = tmdb_id(source)?;
        let endpoint = format!("tv/{id}");
        let images = images_query(&self.settings.current());
        let query = [("append_to_response", "alternative_titles,images"), ("include_image_language", images.as_str())];
        let details: TvDetails = self.get(&endpoint, &query, Some(source)).await?;
        let numbers: Vec<u16> = details.seasons.iter().map(|season| season.season_number).collect();

        let mut seasons = Vec::new();
        for chunk in numbers.chunks(MAX_APPENDED_SEASONS) {
            let append = chunk.iter().map(|number| format!("season/{number}")).collect::<Vec<_>>().join(",");
            let mut page: TvDetails = self.get(&endpoint, &[("append_to_response", &append)], Some(source)).await?;
            for number in chunk {
                let Some(value) = page.appended.remove(&format!("season/{number}")) else { continue };
                let season: SeasonDetails = serde_json::from_value(value).map_err(invalid)?;
                seasons.push(SeasonMetadata {
                    number: season.season_number,
                    episodes: season
                        .episodes
                        .into_iter()
                        .map(|episode| EpisodeMetadata {
                            source_id: episode.id,
                            number: episode.episode_number,
                            title: episode.name,
                            air_date: http::date(episode.air_date.as_deref()),
                        })
                        .collect(),
                });
            }
        }

        Ok(SeriesMetadata {
            source,
            alternate_titles: details.alternative_titles.into_distinct(&details.name, &details.original_name),
            year: http::year(details.first_air_date.as_deref()),
            status: tmdb_wire::source_status(details.status.as_deref()),
            title: details.name,
            original_title: details.original_name,
            artwork: Artwork {
                logo: details.images.logo(self.settings.current().image_language()),
                poster: details.poster_path,
                backdrop: details.backdrop_path,
            },
            seasons,
        })
    }

    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError> {
        let id = tmdb_id(source)?;
        let images = images_query(&self.settings.current());
        let query = [
            ("append_to_response", "release_dates,alternative_titles,images"),
            ("include_image_language", images.as_str()),
        ];
        let details: MovieDetails = self.get(&format!("movie/{id}"), &query, Some(source)).await?;

        Ok(MovieMetadata {
            source,
            year: http::year(details.release_date.as_deref()),
            releases: details.releases(&self.settings.current().region),
            alternate_titles: details.alternative_titles.into_distinct(&details.title, &details.original_title),
            title: details.title,
            original_title: details.original_title,
            artwork: Artwork {
                logo: details.images.logo(self.settings.current().image_language()),
                poster: details.poster_path,
                backdrop: details.backdrop_path,
            },
        })
    }
}

/// Images in the metadata language and images without text, for `include_image_language`.
fn images_query(settings: &MetadataSettings) -> String {
    format!("{},null", settings.image_language())
}

fn tmdb_id(source: ExternalId) -> Result<u64, MetadataError> {
    match source {
        ExternalId::Tmdb(id) => Ok(id),
        ExternalId::Tvdb(_) => Err(MetadataError::Unavailable(format!("TMDB cannot look up {source}").into())),
    }
}
