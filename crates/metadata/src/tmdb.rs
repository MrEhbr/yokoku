use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use yokoku_domain::{EpisodeMetadata, ExternalId, MediaKind, MovieMetadata, Releases, SeasonMetadata, SeriesMetadata};
use yokoku_library::ports::{MetadataError, MetadataProvider, SearchResult};

use crate::wire::{self, MovieDetails, SearchItem, SearchPage, SeasonDetails, TvDetails};

const BASE_URL: &str = "https://api.themoviedb.org/3";
/// TMDB's limit on `append_to_response` entries per request.
const MAX_APPENDED_SEASONS: usize = 20;

/// TMDB API v3 with a read access token.
#[derive(Debug, Clone)]
pub struct TmdbClient {
    http: Client,
    base_url: String,
    token: String,
    language: String,
    region: String,
}

impl TmdbClient {
    /// `language` like `en-US`; `region` like `US` selects movie release dates.
    pub fn new(token: impl Into<String>, language: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: BASE_URL.to_owned(),
            token: token.into(),
            language: language.into(),
            region: region.into(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
        source: Option<ExternalId>,
    ) -> Result<T, MetadataError> {
        let response = self
            .http
            .get(format!("{}/{endpoint}", self.base_url))
            .bearer_auth(&self.token)
            .query(&[("language", self.language.as_str())])
            .query(query)
            .send()
            .await
            .map_err(unavailable)?;

        if let Some(source) = source
            && response.status() == StatusCode::NOT_FOUND
        {
            return Err(MetadataError::NotFound(source));
        }
        response.error_for_status().map_err(unavailable)?.json().await.map_err(unavailable)
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
                    year: wire::year(movie.release_date.as_deref()),
                    title: movie.title,
                    original_title: movie.original_title,
                    poster_path: movie.poster_path,
                }),
                SearchItem::Tv(tv) => Some(SearchResult {
                    kind: MediaKind::Series,
                    source: ExternalId::Tmdb(tv.id),
                    year: wire::year(tv.first_air_date.as_deref()),
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
        let details: TvDetails = self.get(&endpoint, &[], Some(source)).await?;
        let numbers: Vec<u16> = details.seasons.iter().map(|season| season.season_number).collect();

        let mut seasons = Vec::new();
        for chunk in numbers.chunks(MAX_APPENDED_SEASONS) {
            let append = chunk.iter().map(|number| format!("season/{number}")).collect::<Vec<_>>().join(",");
            let mut page: TvDetails = self.get(&endpoint, &[("append_to_response", &append)], Some(source)).await?;
            for number in chunk {
                let Some(value) = page.appended.remove(&format!("season/{number}")) else { continue };
                let season: SeasonDetails = serde_json::from_value(value).map_err(unavailable)?;
                seasons.push(SeasonMetadata {
                    number: season.season_number,
                    episodes: season
                        .episodes
                        .into_iter()
                        .map(|episode| EpisodeMetadata {
                            source_id: episode.id,
                            number: episode.episode_number,
                            title: episode.name,
                            air_date: wire::date(episode.air_date.as_deref()),
                        })
                        .collect(),
                });
            }
        }

        Ok(SeriesMetadata {
            source,
            year: wire::year(details.first_air_date.as_deref()),
            status: wire::source_status(details.status.as_deref()),
            title: details.name,
            original_title: details.original_name,
            alternate_titles: Vec::new(),
            poster_path: details.poster_path,
            seasons,
        })
    }

    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError> {
        let id = tmdb_id(source)?;
        let details: MovieDetails =
            self.get(&format!("movie/{id}"), &[("append_to_response", "release_dates")], Some(source)).await?;

        Ok(MovieMetadata {
            source,
            year: wire::year(details.release_date.as_deref()),
            releases: releases(&details, &self.region),
            title: details.title,
            original_title: details.original_title,
            alternate_titles: Vec::new(),
            poster_path: details.poster_path,
        })
    }
}

/// Earliest date per kind in `region`; the primary release date stands in for a missing cinema date.
fn releases(details: &MovieDetails, region: &str) -> Releases {
    let dates: Vec<_> = details
        .release_dates
        .iter()
        .flat_map(|by_country| &by_country.results)
        .filter(|country| country.iso_3166_1.eq_ignore_ascii_case(region))
        .flat_map(|country| &country.release_dates)
        .filter_map(|release| Some((release.kind, wire::date(Some(&release.release_date))?)))
        .collect();
    let earliest = |kinds: &[u8]| dates.iter().filter(|(kind, _)| kinds.contains(kind)).map(|&(_, date)| date).min();

    Releases {
        cinema: earliest(&[2, 3]).or_else(|| wire::date(details.release_date.as_deref())),
        digital: earliest(&[4]),
        physical: earliest(&[5]),
    }
}

fn tmdb_id(source: ExternalId) -> Result<u64, MetadataError> {
    match source {
        ExternalId::Tmdb(id) => Ok(id),
        ExternalId::Tvdb(_) => Err(MetadataError::Unavailable(format!("TMDB cannot look up {source}").into())),
    }
}

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> MetadataError {
    MetadataError::Unavailable(Box::new(error))
}
