use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use yokoku_core::library::ports::{MetadataError, MetadataProvider, SearchResult};
use yokoku_domain::{
    Artwork, Description, EpisodeMetadata, ExternalId, Live, MediaKind, MovieMetadata, SeasonMetadata, SeriesMetadata,
};

use crate::metadata::{
    MetadataSettings,
    http::{self, Http},
    tmdb_wire::{self, MovieDetails, MovieSummary, SearchItem, SearchPage, SeasonDetails, TvDetails, TvSummary},
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
    async fn search(&self, query: &str, kind: Option<MediaKind>) -> Result<Vec<SearchResult>, MetadataError> {
        let params = [("query", query), ("include_adult", "false")];
        Ok(match kind {
            None => {
                let page: SearchPage<SearchItem> = self.get("search/multi", &params, None).await?;
                page.results
                    .into_iter()
                    .filter_map(|item| match item {
                        SearchItem::Movie(movie) => Some(movie.into()),
                        SearchItem::Tv(tv) => Some(tv.into()),
                        SearchItem::Other => None,
                    })
                    .collect()
            },
            Some(MediaKind::Movie) => {
                let page: SearchPage<MovieSummary> = self.get("search/movie", &params, None).await?;
                page.results.into_iter().map(SearchResult::from).collect()
            },
            Some(MediaKind::Series) => {
                let page: SearchPage<TvSummary> = self.get("search/tv", &params, None).await?;
                page.results.into_iter().map(SearchResult::from).collect()
            },
        })
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        let id = tmdb_id(source)?;
        let endpoint = format!("tv/{id}");
        let images = images_query(&self.settings.current());
        let query = [("append_to_response", "alternative_titles,images"), ("include_image_language", images.as_str())];
        let details: TvDetails = self.get(&endpoint, &query, Some(source)).await?;
        let numbers: Vec<u16> = details.seasons.iter().map(|season| season.season_number).collect();

        let mut seasons = Vec::new();
        let mut runtimes = Vec::new();
        for chunk in numbers.chunks(MAX_APPENDED_SEASONS) {
            let append = chunk.iter().map(|number| format!("season/{number}")).collect::<Vec<_>>().join(",");
            let mut page: TvDetails = self.get(&endpoint, &[("append_to_response", &append)], Some(source)).await?;
            for number in chunk {
                let Some(value) = page.appended.remove(&format!("season/{number}")) else { continue };
                let season: SeasonDetails =
                    serde_json::from_value(value).map_err(|error| MetadataError::Invalid(error.into()))?;
                runtimes.extend(season.episodes.iter().filter_map(|episode| episode.runtime));
                seasons.push(SeasonMetadata {
                    number: season.season_number,
                    episodes: season
                        .episodes
                        .into_iter()
                        .map(|episode| EpisodeMetadata {
                            source_id: episode.id,
                            number: episode.episode_number,
                            title: episode.name,
                            overview: episode.overview,
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
            description: Description {
                overview: details.overview,
                genres: details.genres.into_iter().map(|genre| genre.name).collect(),
                runtime: details.episode_run_time.first().copied().or_else(|| most_common(runtimes)),
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
            description: Description {
                overview: details.overview,
                genres: details.genres.into_iter().map(|genre| genre.name).collect(),
                runtime: details.runtime.filter(|&minutes| minutes > 0),
            },
        })
    }
}

impl From<MovieSummary> for SearchResult {
    fn from(movie: MovieSummary) -> Self {
        Self {
            kind: MediaKind::Movie,
            source: ExternalId::Tmdb(movie.id),
            year: http::year(movie.release_date.as_deref()),
            title: movie.title,
            original_title: movie.original_title,
            poster_path: movie.poster_path,
            overview: movie.overview,
        }
    }
}

impl From<TvSummary> for SearchResult {
    fn from(tv: TvSummary) -> Self {
        Self {
            kind: MediaKind::Series,
            source: ExternalId::Tmdb(tv.id),
            year: http::year(tv.first_air_date.as_deref()),
            title: tv.name,
            original_title: tv.original_name,
            poster_path: tv.poster_path,
            overview: tv.overview,
        }
    }
}

/// The value that occurs most often; the smallest of equally common ones.
fn most_common(values: Vec<u16>) -> Option<u16> {
    let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
    for value in values.into_iter().filter(|&value| value > 0) {
        *counts.entry(value).or_default() += 1;
    }
    counts.into_iter().rev().max_by_key(|&(_, count)| count).map(|(value, _)| value)
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
