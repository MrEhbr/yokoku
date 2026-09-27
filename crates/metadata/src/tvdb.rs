use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tracing::debug;
use yokoku_domain::{EpisodeMetadata, ExternalId, MediaKind, MovieMetadata, SeasonMetadata, SeriesMetadata};
use yokoku_library::ports::{MetadataError, MetadataProvider, SearchResult};

use crate::{
    http::{self, Http},
    tvdb_wire::{self, Envelope, EpisodePage, Login, SearchItem, SeriesDetails, Token},
    wire,
};

const BASE_URL: &str = "https://api4.thetvdb.com/v4";
/// 500 episodes a page.
const MAX_EPISODE_PAGES: usize = 100;

/// TheTVDB API v4 for series, with a project API key and, for user-supported keys, a subscriber PIN.
pub struct TvdbClient {
    http: Http,
    base_url: String,
    api_key: String,
    pin: Option<String>,
    language: String,
    token: Mutex<Option<String>>,
}

impl TvdbClient {
    /// `language` is a three-letter code like `eng`.
    pub fn new(api_key: impl Into<String>, pin: Option<String>, language: impl Into<String>) -> Self {
        Self {
            http: Http::new("TVDB"),
            base_url: BASE_URL.to_owned(),
            api_key: api_key.into(),
            pin,
            language: language.into(),
            token: Mutex::new(None),
        }
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Logs in on first use, and once more when the token is refused.
    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
        source: Option<ExternalId>,
    ) -> Result<Envelope<T>, MetadataError> {
        let request = self.http.get(format!("{}/{endpoint}", self.base_url)).query(query);
        let token = self.token().await?;
        let response =
            match self.http.send(request.try_clone().expect("GET has no body").bearer_auth(&token), source).await {
                Err(MetadataError::Refused(_)) => {
                    debug!("renewing the TVDB token");
                    self.forget(&token).await;
                    self.http.send(request.bearer_auth(self.token().await?), source).await?
                },
                response => response?,
            };
        http::json(response).await
    }

    /// The cached token, or a new one; concurrent callers wait for a single login.
    async fn token(&self) -> Result<String, MetadataError> {
        let mut token = self.token.lock().await;
        if let Some(token) = token.as_ref() {
            return Ok(token.clone());
        }
        let login = Login { apikey: &self.api_key, pin: self.pin.as_deref() };
        let request = self.http.post(format!("{}/login", self.base_url)).json(&login);
        let envelope: Envelope<Token> = http::json(self.http.send(request, None).await?).await?;
        Ok(token.insert(envelope.data.token).clone())
    }

    /// Drops `stale` unless another caller already replaced it.
    async fn forget(&self, stale: &str) {
        let mut token = self.token.lock().await;
        if token.as_deref() == Some(stale) {
            *token = None;
        }
    }

    async fn episodes(&self, id: u64, source: ExternalId) -> Result<Vec<tvdb_wire::EpisodeItem>, MetadataError> {
        let endpoint = format!("series/{id}/episodes/default/{}", self.language);
        let mut episodes = Vec::new();
        for page in 0..MAX_EPISODE_PAGES {
            let page = page.to_string();
            let envelope: Envelope<EpisodePage> = self.get(&endpoint, &[("page", &page)], Some(source)).await?;
            episodes.extend(envelope.data.episodes);
            if envelope.links.next.is_none() {
                return Ok(episodes);
            }
        }
        Err(MetadataError::Invalid(format!("{source} has more than {MAX_EPISODE_PAGES} episode pages").into()))
    }
}

#[async_trait]
impl MetadataProvider for TvdbClient {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, MetadataError> {
        let envelope: Envelope<Vec<SearchItem>> =
            self.get("search", &[("query", query), ("type", "series")], None).await?;

        Ok(envelope
            .data
            .into_iter()
            .filter_map(|mut item| {
                let id = item.tvdb_id.parse().ok()?;
                Some(SearchResult {
                    kind: MediaKind::Series,
                    source: ExternalId::Tvdb(id),
                    title: item.translations.remove(&self.language).unwrap_or_else(|| item.name.clone()),
                    original_title: item.name,
                    year: wire::year(item.year.as_deref()),
                    poster_path: item.image_url,
                })
            })
            .collect())
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        let id = tvdb_id(source)?;
        let details: SeriesDetails = self
            .get(&format!("series/{id}/extended"), &[("meta", "translations"), ("short", "true")], Some(source))
            .await?
            .data;

        let mut seasons: BTreeMap<u16, Vec<EpisodeMetadata>> = BTreeMap::new();
        for episode in self.episodes(id, source).await? {
            seasons.entry(episode.season_number).or_default().push(EpisodeMetadata {
                source_id: episode.id,
                number: episode.number,
                title: episode.name.unwrap_or_default(),
                air_date: wire::date(episode.aired.as_deref()),
            });
        }
        let seasons = seasons
            .into_iter()
            .map(|(number, mut episodes)| {
                episodes.sort_by_key(|episode| episode.number);
                SeasonMetadata { number, episodes }
            })
            .collect();

        let title = details.translated_name(&self.language).unwrap_or(&details.name).to_owned();
        let mut alternate_titles: Vec<String> = Vec::new();
        for alias in details.aliases {
            if alias.name != title && alias.name != details.name && !alternate_titles.contains(&alias.name) {
                alternate_titles.push(alias.name);
            }
        }

        Ok(SeriesMetadata {
            source,
            year: wire::year(details.year.as_deref()),
            status: tvdb_wire::source_status(details.status.and_then(|status| status.name).as_deref()),
            title,
            original_title: details.name,
            alternate_titles,
            poster_path: details.image,
            seasons,
        })
    }

    async fn movie(&self, source: ExternalId) -> Result<MovieMetadata, MetadataError> {
        Err(MetadataError::Unavailable(format!("TVDB does not look up movies, such as {source}").into()))
    }
}

fn tvdb_id(source: ExternalId) -> Result<u64, MetadataError> {
    match source {
        ExternalId::Tvdb(id) => Ok(id),
        ExternalId::Tmdb(_) => Err(MetadataError::Unavailable(format!("TVDB cannot look up {source}").into())),
    }
}
