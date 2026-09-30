use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tracing::debug;
use yokoku_core::library::ports::{MetadataError, MetadataProvider, SearchResult};
use yokoku_domain::{
    Artwork, Description, EpisodeMetadata, ExternalId, Live, MediaKind, MovieMetadata, SeasonMetadata, SeriesMetadata,
};

use crate::metadata::{
    MetadataSettings,
    http::{self, Http},
    tvdb_wire::{
        self, Envelope, EpisodePage, Login, SERIES_BACKGROUND, SERIES_CLEAR_LOGO, SearchItem, SeriesArtworks,
        SeriesDetails, Token,
    },
};

/// 500 episodes a page.
const MAX_EPISODE_PAGES: usize = 100;

/// TheTVDB API v4 for series, with a project API key and, for user-supported keys, a subscriber PIN.
pub struct TvdbClient {
    http: Http,
    settings: Live<MetadataSettings>,
    /// The token with the API key and PIN it was issued for.
    token: Mutex<Option<(Credentials, String)>>,
    max_episode_pages: usize,
}

#[derive(PartialEq, Eq)]
struct Credentials {
    api_key: String,
    pin: Option<String>,
}

impl TvdbClient {
    pub fn new(settings: Live<MetadataSettings>) -> Self {
        Self { http: Http::new("TVDB"), settings, token: Mutex::new(None), max_episode_pages: MAX_EPISODE_PAGES }
    }

    /// Logs in on first use, and once more when the token is refused.
    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
        source: Option<ExternalId>,
    ) -> Result<Envelope<T>, MetadataError> {
        let settings = self.settings.current();
        let request = self.http.get(format!("{}/{endpoint}", settings.tvdb.url)).query(query);
        let token = self.token(&settings).await?;
        let response =
            match self.http.send(request.try_clone().expect("GET has no body").bearer_auth(&token), source).await {
                Err(MetadataError::Refused(_)) => {
                    debug!("renewing the TVDB token");
                    self.forget(&token).await;
                    self.http.send(request.bearer_auth(self.token(&settings).await?), source).await?
                },
                response => response?,
            };
        http::json(response).await
    }

    /// The cached token, or a new one once the API key or PIN changed; concurrent callers wait for
    /// a single login.
    async fn token(&self, settings: &MetadataSettings) -> Result<String, MetadataError> {
        let api_key = settings.tvdb.api_key.as_ref().ok_or_else(|| MetadataError::Refused("no TVDB API key".into()))?;
        let credentials = Credentials {
            api_key: api_key.expose().to_owned(),
            pin: settings.tvdb.pin.as_ref().map(|pin| pin.expose().to_owned()),
        };
        let mut cached = self.token.lock().await;
        if let Some((issued_for, token)) = cached.as_ref()
            && *issued_for == credentials
        {
            return Ok(token.clone());
        }
        let login = Login { apikey: &credentials.api_key, pin: credentials.pin.as_deref() };
        let request = self.http.post(format!("{}/login", settings.tvdb.url)).json(&login);
        let envelope: Envelope<Token> = http::json(self.http.send(request, None).await?).await?;
        *cached = Some((credentials, envelope.data.token.clone()));
        Ok(envelope.data.token)
    }

    /// Drops `stale` unless another caller already replaced it.
    async fn forget(&self, stale: &str) {
        let mut cached = self.token.lock().await;
        if cached.as_ref().is_some_and(|(_, token)| token == stale) {
            *cached = None;
        }
    }

    /// The three-letter code for the configured language, e.g. `eng`.
    fn language(&self) -> Result<&'static str, MetadataError> {
        self.settings.current().tvdb_language().map_err(|error| MetadataError::Unavailable(Box::new(error)))
    }

    async fn episodes(
        &self,
        id: u64,
        source: ExternalId,
        language: &str,
    ) -> Result<Vec<tvdb_wire::EpisodeItem>, MetadataError> {
        let endpoint = format!("series/{id}/episodes/default/{language}");
        let mut episodes = Vec::new();
        for page in 0..self.max_episode_pages {
            let page = page.to_string();
            let envelope: Envelope<EpisodePage> = self.get(&endpoint, &[("page", &page)], Some(source)).await?;
            episodes.extend(envelope.data.episodes);
            if envelope.links.next.is_none() {
                return Ok(episodes);
            }
        }
        Err(MetadataError::Invalid(format!("{source} has more than {} episode pages", self.max_episode_pages).into()))
    }
}

#[async_trait]
impl MetadataProvider for TvdbClient {
    /// Series only; Yokoku takes movies from TMDB.
    async fn search(&self, query: &str, kind: Option<MediaKind>) -> Result<Vec<SearchResult>, MetadataError> {
        if kind == Some(MediaKind::Movie) {
            return Ok(Vec::new());
        }
        let language = self.language()?;
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
                    title: item.translations.remove(language).unwrap_or_else(|| item.name.clone()),
                    original_title: item.name,
                    year: item.year,
                    poster_path: item.image_url,
                    overview: item.overviews.remove(language).or(item.overview).unwrap_or_default(),
                })
            })
            .collect())
    }

    async fn series(&self, source: ExternalId) -> Result<SeriesMetadata, MetadataError> {
        let id = tvdb_id(source)?;
        let language = self.language()?;
        let details: SeriesDetails = self
            .get(&format!("series/{id}/extended"), &[("meta", "translations"), ("short", "true")], Some(source))
            .await?
            .data;
        let artworks: SeriesArtworks = self.get(&format!("series/{id}/artworks"), &[], Some(source)).await?.data;

        let mut seasons: BTreeMap<u16, Vec<EpisodeMetadata>> = BTreeMap::new();
        for episode in self.episodes(id, source, language).await? {
            seasons.entry(episode.season_number).or_default().push(EpisodeMetadata {
                source_id: episode.id,
                number: episode.number,
                title: episode.name.unwrap_or_default(),
                overview: episode.overview.unwrap_or_default(),
                air_date: episode.aired,
            });
        }
        let seasons = seasons
            .into_iter()
            .map(|(number, mut episodes)| {
                episodes.sort_by_key(|episode| episode.number);
                SeasonMetadata { number, episodes }
            })
            .collect();

        let description = Description {
            overview: details.overview(language).unwrap_or_default().to_owned(),
            genres: details.genres.iter().map(|genre| genre.name.clone()).collect(),
            runtime: details.average_runtime.filter(|&minutes| minutes > 0),
        };
        let title = details.translated_name(language).unwrap_or(&details.name).to_owned();
        let mut alternate_titles: Vec<String> = Vec::new();
        for alias in details.aliases {
            if alias.name != title && alias.name != details.name && !alternate_titles.contains(&alias.name) {
                alternate_titles.push(alias.name);
            }
        }

        Ok(SeriesMetadata {
            source,
            year: details.year,
            status: tvdb_wire::source_status(details.status.and_then(|status| status.name).as_deref()),
            title,
            original_title: details.name,
            alternate_titles,
            artwork: Artwork {
                poster: details.image,
                backdrop: artworks.best(SERIES_BACKGROUND, [None, Some(language)]),
                logo: artworks.best(SERIES_CLEAR_LOGO, [Some(language), None]),
            },
            description,
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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};
    use yokoku_core::library::ports::MetadataError;
    use yokoku_domain::{ExternalId, Live, Secret};

    use super::TvdbClient;
    use crate::metadata::{MetadataSettings, TvdbSettings};

    #[tokio::test]
    async fn endless_episode_pages_are_invalid() {
        let server = MockServer::start().await;
        let answer = |body| ResponseTemplate::new(200).set_body_json(body);
        Mock::given(path("/login"))
            .respond_with(answer(json!({ "status": "success", "data": { "token": "token" } })))
            .mount(&server)
            .await;
        Mock::given(path("/series/424536/episodes/default/eng"))
            .respond_with(answer(json!({ "data": { "episodes": [] }, "links": { "next": "more" } })))
            .expect(2)
            .mount(&server)
            .await;
        let tvdb = TvdbSettings { api_key: Some(Secret::new("key")), pin: None, url: server.uri() };
        let settings = Live::fixed(MetadataSettings { tvdb, ..MetadataSettings::default() });
        let client = TvdbClient { max_episode_pages: 2, ..TvdbClient::new(settings) };

        let error = client.episodes(424536, ExternalId::Tvdb(424536), "eng").await.unwrap_err();

        assert!(matches!(error, MetadataError::Invalid(_)), "{error:?}");
    }
}
