use std::time::Duration;

use reqwest::{StatusCode, header::LOCATION, redirect};
use serde::de::DeserializeOwned;
use yokoku_core::downloads::ports::{IndexerError, Release, ReleaseQuery, TorrentSource};
use yokoku_domain::{MediaKind, Secret};

use super::{TorznabFeed, torznab_wire as wire};

const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TORRENT: usize = 10_000_000;

pub struct TorznabClient {
    http: reqwest::Client,
}

impl Default for TorznabClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TorznabClient {
    pub fn new() -> Self {
        crate::tls::install_crypto_provider();
        let redirects = redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() == "magnet" {
                attempt.stop()
            } else if attempt.previous().len() >= 5 {
                attempt.error("too many redirects")
            } else {
                attempt.follow()
            }
        });
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(TIMEOUT)
                .redirect(redirects)
                .build()
                .expect("TLS backend initializes"),
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        feed: &TorznabFeed,
        params: &[(&str, String)],
    ) -> Result<T, IndexerError> {
        let key = feed.api_key.as_ref().map(Secret::expose);
        let request = self.http.get(&feed.url).query(params);
        let request = if let Some(key) = key { request.query(&[("apikey", key)]) } else { request };
        let response = request.send().await.map_err(|_| IndexerError::Unavailable("Torznab request failed".into()))?;
        if !response.status().is_success() {
            return Err(IndexerError::Unavailable(format!("Torznab answered HTTP {}", response.status()).into()));
        }
        let body = response.text().await.map_err(|_| IndexerError::Unavailable("Torznab response failed".into()))?;
        quick_xml::de::from_str(&body).map_err(|_| IndexerError::Refused("Torznab returned invalid XML".into()))
    }

    pub async fn test(&self, feed: &TorznabFeed) -> Result<String, IndexerError> {
        let caps: wire::Caps = self.request(feed, &[("t", "caps".into())]).await?;
        if let Some(reason) = caps.error {
            return Err(IndexerError::Refused(reason));
        }
        let searching =
            caps.searching.ok_or_else(|| IndexerError::Refused("Torznab lists no search capabilities".into()))?;
        if ![searching.search, searching.tv, searching.movie].into_iter().flatten().any(|mode| mode.supports("q")) {
            return Err(IndexerError::Refused("Torznab does not support text search".into()));
        }
        Ok(caps.server.and_then(|server| server.title).unwrap_or_else(|| feed.name.clone()))
    }

    pub async fn search(&self, feed: &TorznabFeed, query: &ReleaseQuery) -> Result<Vec<Release>, IndexerError> {
        // Caps discovery and the feed request share one deadline.
        let search = async {
            let caps: wire::Caps = self.request(feed, &[("t", "caps".into())]).await?;
            if let Some(reason) = caps.error {
                return Err(IndexerError::Refused(reason));
            }
            let searching =
                caps.searching.ok_or_else(|| IndexerError::Refused("Torznab lists no search capabilities".into()))?;
            let preferred = match query.kind {
                Some(MediaKind::Series) => {
                    searching.tv.as_ref().filter(|mode| mode.supports("q")).map(|mode| ("tvsearch", mode))
                },
                Some(MediaKind::Movie) => {
                    searching.movie.as_ref().filter(|mode| mode.supports("q")).map(|mode| ("movie", mode))
                },
                None => None,
            };
            let (mode, supports_season) = match preferred {
                Some((mode, capabilities)) => (mode, capabilities.supports("season")),
                None if searching.search.as_ref().is_some_and(|mode| mode.supports("q")) => ("search", false),
                None => return Err(IndexerError::Refused("Torznab does not support this text search".into())),
            };
            let text = if query.kind == Some(MediaKind::Series) && query.season.is_some() && !supports_season {
                format!("{} S{:02}", query.text, query.season.unwrap_or_default())
            } else {
                query.text.clone()
            };
            let mut params = vec![("t", mode.to_owned()), ("q", text)];
            if supports_season {
                params.extend(query.season.map(|season| ("season", season.to_string())));
            }
            let result: wire::Feed = self.request(feed, &params).await?;
            if let Some(reason) = result.error {
                return Err(IndexerError::Refused(reason));
            }
            Ok(result
                .channel
                .into_iter()
                .flat_map(|channel| channel.items)
                .filter_map(|item| {
                    let mut release = Release::try_from(item).ok()?;
                    release.tracker = feed.name.clone();
                    Some(release)
                })
                .collect())
        };
        tokio::time::timeout(TIMEOUT, search)
            .await
            .map_err(|_| IndexerError::Unavailable("Torznab search timed out".into()))?
    }

    pub async fn fetch(&self, link: &str) -> Result<TorrentSource, IndexerError> {
        if link.starts_with("magnet:") {
            return Ok(TorrentSource::Magnet(link.to_owned()));
        }
        let url = reqwest::Url::parse(link).map_err(|_| IndexerError::Refused("Invalid torrent URL".into()))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(IndexerError::Refused("Invalid torrent URL".into()));
        }
        let mut response =
            self.http.get(url).send().await.map_err(|_| IndexerError::Unavailable("Torrent download failed".into()))?;
        if response.status().is_redirection()
            && let Some(magnet) = response.headers().get(LOCATION).and_then(|value| value.to_str().ok())
            && magnet.starts_with("magnet:")
        {
            return Ok(TorrentSource::Magnet(magnet.into()));
        }
        if response.status() != StatusCode::OK {
            return Err(IndexerError::Unavailable(
                format!("Torrent download answered HTTP {}", response.status()).into(),
            ));
        }
        if response.content_length().is_some_and(|length| length > MAX_TORRENT as u64) {
            return Err(IndexerError::Refused("Torrent file is too large".into()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) =
            response.chunk().await.map_err(|_| IndexerError::Unavailable("Torrent download failed".into()))?
        {
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
