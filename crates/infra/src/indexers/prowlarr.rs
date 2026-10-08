use std::time::Duration;

use async_trait::async_trait;
use futures_util::future::join_all;
use reqwest::{StatusCode, Url, header::LOCATION, redirect};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::warn;
use yokoku_core::downloads::ports::{
    Indexer, IndexerError, Release, ReleaseQuery, SearchResult, TorrentSource, Tracker,
};
use yokoku_domain::{Live, MediaKind, Secret, Trackers};

use super::torznab_wire as wire;

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TORRENT: usize = 10_000_000;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProwlarrSettings {
    /// Server address, e.g. `http://localhost:9696`; search is off while unset.
    pub url: Option<String>,
    pub api_key: Option<Secret>,
}

/// Searches enabled torrent indexers through Prowlarr's per-indexer Torznab API.
pub struct ProwlarrClient {
    http: reqwest::Client,
    settings: Live<ProwlarrSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedIndexer {
    id: u32,
    name: String,
    enable: bool,
    supports_search: bool,
    protocol: String,
}

impl ProwlarrClient {
    pub fn new(settings: Live<ProwlarrSettings>) -> Self {
        crate::tls::install_crypto_provider();
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(TIMEOUT)
            .redirect(redirect::Policy::none())
            .build()
            .expect("TLS backend initializes");
        Self { http, settings }
    }

    fn server(&self) -> Result<(String, String), IndexerError> {
        let settings = self.settings.current();
        let url = settings.url.as_deref().ok_or(IndexerError::NotConfigured)?.trim_end_matches('/').to_owned();
        let key = settings.api_key.as_ref().ok_or_else(|| IndexerError::Refused("Set the Prowlarr API key".into()))?;
        Ok((url, key.expose().to_owned()))
    }

    async fn indexers(&self) -> Result<Vec<ListedIndexer>, IndexerError> {
        let (url, key) = self.server()?;
        let response = self
            .http
            .get(format!("{url}/api/v1/indexer"))
            .header("X-Api-Key", key)
            .send()
            .await
            .map_err(|error| IndexerError::Unavailable(error.into()))?;
        if !response.status().is_success() {
            return Err(IndexerError::Refused(format!("Prowlarr answered HTTP {}", response.status())));
        }
        response.json().await.map_err(|error| IndexerError::Unavailable(error.into()))
    }

    async fn torznab<T: DeserializeOwned>(&self, id: u32, params: &[(&str, String)]) -> Result<T, IndexerError> {
        let (url, key) = self.server()?;
        let response = self
            .http
            .get(format!("{url}/api/v1/indexer/{id}/newznab"))
            .header("X-Api-Key", key)
            .query(params)
            .send()
            .await
            .map_err(|error| IndexerError::Unavailable(error.into()))?;
        if !response.status().is_success() {
            return Err(IndexerError::Refused(format!("Prowlarr indexer answered HTTP {}", response.status())));
        }
        let body = response.text().await.map_err(|error| IndexerError::Unavailable(error.into()))?;
        quick_xml::de::from_str(&body).map_err(|error| IndexerError::Unavailable(error.into()))
    }

    async fn releases(&self, indexer: &ListedIndexer, query: &ReleaseQuery) -> Result<Vec<Release>, IndexerError> {
        let caps: wire::Caps = self.torznab(indexer.id, &[("t", "caps".into())]).await?;
        if let Some(error) = caps.error {
            return Err(IndexerError::Refused(error));
        }
        let searching =
            caps.searching.ok_or_else(|| IndexerError::Refused("Prowlarr lists no search capabilities".into()))?;
        let preferred = match query.kind {
            Some(MediaKind::Series) => {
                searching.tv.as_ref().filter(|mode| mode.supports("q")).map(|mode| ("tvsearch", mode))
            },
            Some(MediaKind::Movie) => {
                searching.movie.as_ref().filter(|mode| mode.supports("q")).map(|mode| ("movie", mode))
            },
            None => None,
        };
        let (function, supports_season) = match preferred {
            Some((function, mode)) => (function, mode.supports("season")),
            None if searching.search.as_ref().is_some_and(|mode| mode.supports("q")) => ("search", false),
            None => return Err(IndexerError::Refused(format!("{} does not support text search", indexer.name))),
        };
        let text = if query.kind == Some(MediaKind::Series) && query.season.is_some() && !supports_season {
            format!("{} S{:02}", query.text, query.season.unwrap_or_default())
        } else {
            query.text.clone()
        };
        let mut params = vec![("t", function.to_owned()), ("q", text)];
        if supports_season {
            params.extend(query.season.map(|season| ("season", season.to_string())));
            params.extend(query.season.and(query.episode).map(|episode| ("ep", episode.to_string())));
        }
        let feed: wire::Feed = self.torznab(indexer.id, &params).await?;
        if let Some(error) = feed.error {
            return Err(IndexerError::Refused(error));
        }
        Ok(feed
            .channel
            .into_iter()
            .flat_map(|channel| channel.items)
            .filter_map(|item| {
                let mut release = Release::try_from(item).ok()?;
                release.tracker = indexer.name.clone();
                Some(release)
            })
            .collect())
    }
}

#[async_trait]
impl Indexer for ProwlarrClient {
    async fn version(&self) -> Result<String, IndexerError> {
        self.indexers().await?;
        Ok("Prowlarr".into())
    }

    async fn trackers(&self) -> Result<Vec<Tracker>, IndexerError> {
        Ok(self
            .indexers()
            .await?
            .into_iter()
            .filter(|indexer| indexer.enable && indexer.supports_search && indexer.protocol == "torrent")
            .map(|indexer| Tracker {
                id: indexer.id.to_string().parse().expect("numeric indexer id is valid"),
                name: indexer.name,
            })
            .collect())
    }

    async fn search(&self, query: &ReleaseQuery) -> Result<SearchResult, IndexerError> {
        let listed = self.indexers().await?;
        let selected: Vec<_> = listed
            .iter()
            .filter(|indexer| indexer.enable && indexer.supports_search && indexer.protocol == "torrent")
            .filter(|indexer| match &query.trackers {
                Trackers::All => true,
                Trackers::Only(set) => set.ids().iter().any(|id| id.as_str() == indexer.id.to_string()),
            })
            .collect();
        if selected.is_empty() {
            return Err(IndexerError::Refused("Add or select a searchable torrent indexer in Prowlarr".into()));
        }
        let results = join_all(selected.iter().map(|indexer| async move {
            tokio::time::timeout(TIMEOUT, self.releases(indexer, query))
                .await
                .map_err(|_| IndexerError::Unavailable("Prowlarr search timed out".into()))?
        }))
        .await;
        let mut releases = Vec::new();
        let mut warnings = Vec::new();
        let mut last_error = None;
        for (indexer, result) in selected.iter().zip(results) {
            match result {
                Ok(found) => releases.extend(found),
                Err(error) => {
                    warn!(name = %indexer.name, %error, "Prowlarr tracker search failed");
                    warnings.push(format!("{} · Prowlarr could not be searched", indexer.name));
                    last_error = Some(error);
                },
            }
        }
        if warnings.len() == selected.len() {
            return Err(last_error.expect("at least one search failed"));
        }
        Ok(SearchResult { releases, warnings })
    }

    async fn fetch(&self, link: &str) -> Result<TorrentSource, IndexerError> {
        if link.starts_with("magnet:") {
            return Ok(TorrentSource::Magnet(link.to_owned()));
        }
        let (base, key) = self.server()?;
        let url = Url::parse(link).map_err(|_| IndexerError::Refused("Invalid Prowlarr download link".into()))?;
        let prefix = format!("{base}/api/v1/indexer/");
        let path = link
            .strip_prefix(&prefix)
            .ok_or_else(|| IndexerError::Refused("The link does not lead to Prowlarr".into()))?;
        let (id, rest) =
            path.split_once('/').ok_or_else(|| IndexerError::Refused("Invalid Prowlarr download link".into()))?;
        if id.parse::<u32>().is_err() || !rest.starts_with("download?") {
            return Err(IndexerError::Refused("Invalid Prowlarr download link".into()));
        }
        let mut response = self
            .http
            .get(url)
            .header("X-Api-Key", key)
            .send()
            .await
            .map_err(|error| IndexerError::Unavailable(error.into()))?;
        if response.status().is_redirection() {
            if let Some(magnet) = response.headers().get(LOCATION).and_then(|value| value.to_str().ok())
                && magnet.starts_with("magnet:")
            {
                return Ok(TorrentSource::Magnet(magnet.into()));
            }
            return Err(IndexerError::Refused(
                "Prowlarr redirected the download; turn off Redirect for this indexer".into(),
            ));
        }
        if response.status() != StatusCode::OK {
            return Err(IndexerError::Refused(format!("Prowlarr download answered HTTP {}", response.status())));
        }
        if response.content_length().is_some_and(|length| length > MAX_TORRENT as u64) {
            return Err(IndexerError::Refused("Torrent file is too large".into()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| IndexerError::Unavailable(error.into()))? {
            if bytes.len() + chunk.len() > MAX_TORRENT {
                return Err(IndexerError::Refused("Torrent file is too large".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.first() != Some(&b'd') {
            return Err(IndexerError::Refused("The indexer sent no torrent".into()));
        }
        Ok(TorrentSource::File(bytes))
    }
}
