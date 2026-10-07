use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::future::join_all;
use reqwest::{StatusCode, Url, header::LOCATION, redirect};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::{debug, warn};
use yokoku_core::downloads::ports::{
    Indexer, IndexerError, Release, ReleaseQuery, SearchResult, TorrentSource, Tracker,
};
use yokoku_domain::{Live, MediaKind, Secret, TrackerId, Trackers};

use crate::indexers::torznab_wire as wire;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Jackett answers once every tracker it searches has answered or timed out.
const TIMEOUT: Duration = Duration::from_secs(60);
/// The Torznab feed that searches every configured tracker.
const ALL: &str = "all";
/// The query parameter of a Jackett download link that carries the API key.
const LINK_KEY: &str = "jackett_apikey";
const MAX_REDIRECTS: usize = 5;
/// The Torznab error code for a search function or parameter no tracker supports.
const UNSUPPORTED: u16 = 201;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JackettSettings {
    /// Server address, e.g. `http://localhost:9117`; search is off while unset.
    pub url: Option<String>,
    pub api_key: Option<Secret>,
}

/// Searches the trackers configured in Jackett: all through its aggregate Torznab feed, or chosen
/// ones through their own feeds at once.
pub struct JackettClient {
    http: reqwest::Client,
    settings: Live<JackettSettings>,
}

impl JackettClient {
    pub fn new(settings: Live<JackettSettings>) -> Self {
        Self::with_timeout(settings, TIMEOUT)
    }

    fn with_timeout(settings: Live<JackettSettings>, timeout: Duration) -> Self {
        crate::tls::install_crypto_provider();
        let policy = redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() == "magnet" {
                attempt.stop()
            } else if attempt.previous().len() > MAX_REDIRECTS {
                attempt.error("too many redirects")
            } else {
                attempt.follow()
            }
        });
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .redirect(policy)
            .build()
            .expect("TLS backend initializes");
        Self { http, settings }
    }

    /// The configured address without a trailing slash, and the API key.
    fn server(&self) -> Result<(String, String), IndexerError> {
        let settings = self.settings.current();
        let url = settings.url.as_deref().ok_or(IndexerError::NotConfigured)?.trim_end_matches('/').to_owned();
        Ok((url, settings.api_key.as_ref().map_or("", Secret::expose).to_owned()))
    }

    /// A call to the Torznab feed of `tracker`, or of every tracker without one, with `params`, as `T`.
    async fn torznab<T: DeserializeOwned>(
        &self,
        tracker: Option<&TrackerId>,
        params: &[(&str, String)],
    ) -> Result<T, IndexerError> {
        let (url, api_key) = self.server()?;
        let feed = tracker.map_or(ALL, TrackerId::as_str);
        let started = Instant::now();
        let response = self
            .http
            .get(format!("{url}/api/v2.0/indexers/{feed}/results/torznab/api"))
            .query(&[("apikey", api_key)])
            .query(params)
            .send()
            .await
            .map_err(|error| IndexerError::Unavailable(error.into()))?;
        let status = response.status();
        debug!(feed, status = status.as_u16(), elapsed_ms = started.elapsed().as_millis(), "Jackett search");
        if !status.is_success() {
            return Err(IndexerError::Unavailable(format!("HTTP {status}").into()));
        }
        let body = response.text().await.map_err(|error| IndexerError::Unavailable(error.into()))?;
        quick_xml::de::from_str(&body).map_err(|error| IndexerError::Unavailable(error.into()))
    }

    /// The releases `tracker`, or every tracker without one, finds with `params`; `searches` names
    /// the search in an error.
    async fn releases(
        &self,
        tracker: Option<&TrackerId>,
        params: &[(&str, String)],
        searches: &str,
    ) -> Result<Vec<Release>, IndexerError> {
        let feed: wire::Feed = self.torznab(tracker, params).await?;
        match (feed.code, feed.error, tracker) {
            (Some(UNSUPPORTED), _, None) => {
                return Err(IndexerError::Refused(format!("no tracker added in Jackett supports {searches}")));
            },
            (Some(UNSUPPORTED), _, Some(tracker)) => {
                return Err(IndexerError::Refused(format!("{tracker} doesn't support {searches}")));
            },
            (_, Some(error), _) => return Err(IndexerError::Refused(error)),
            _ => {},
        }
        let items = feed.channel.map(|channel| channel.items).unwrap_or_default();
        Ok(items.into_iter().filter_map(|item| Release::try_from(item).ok()).collect())
    }
}

#[async_trait]
impl Indexer for JackettClient {
    async fn version(&self) -> Result<String, IndexerError> {
        let caps: wire::Caps = self.torznab(None, &[("t", "caps".to_owned())]).await?;
        if let Some(error) = caps.error {
            return Err(IndexerError::Refused(error));
        }
        let server = caps.server.ok_or_else(|| IndexerError::Unavailable("caps without a server".into()))?;
        let title = server.title.unwrap_or_else(|| "Jackett".to_owned());
        Ok(server.version.map_or_else(|| title.clone(), |version| format!("{title} {version}")))
    }

    async fn search(&self, query: &ReleaseQuery) -> Result<SearchResult, IndexerError> {
        // Without `cat`: Jackett files some trackers' TV and movie releases, like Rutor's, under Other.
        let (function, searches) = match query.kind {
            Some(MediaKind::Series) => ("tvsearch", "TV searches"),
            Some(MediaKind::Movie) => ("movie", "movie searches"),
            None => ("search", "searches"),
        };
        let mut params = vec![("t", function.to_owned()), ("q", query.text.clone())];
        if query.kind == Some(MediaKind::Series) {
            params.extend(query.season.map(|season| ("season", season.to_string())));
            params.extend(query.season.and(query.episode).map(|episode| ("ep", episode.to_string())));
        }
        let chosen = match &query.trackers {
            Trackers::All => {
                return self
                    .releases(None, &params, searches)
                    .await
                    .map(|releases| SearchResult { releases, warnings: Vec::new() });
            },
            Trackers::Only(set) => set.ids(),
        };
        let searched = join_all(chosen.iter().map(|tracker| self.releases(Some(tracker), &params, searches))).await;
        let mut releases = Vec::new();
        let mut warnings = Vec::new();
        let mut failed = 0;
        let mut last_error = None;
        for (tracker, result) in chosen.iter().zip(searched) {
            match result {
                Ok(found) => releases.extend(found),
                Err(error) => {
                    warn!(%tracker, %error, "a tracker search failed; the other trackers' releases are kept");
                    warnings.push(format!("Jackett tracker {tracker} could not be searched"));
                    failed += 1;
                    last_error = Some(error);
                },
            }
        }
        match last_error {
            Some(error) if failed == chosen.len() => Err(error),
            _ => Ok(SearchResult { releases, warnings }),
        }
    }

    /// The configured trackers; one whose id is not a [`TrackerId`] is left out.
    async fn trackers(&self) -> Result<Vec<Tracker>, IndexerError> {
        let params = [("t", "indexers".to_owned()), ("configured", "true".to_owned())];
        let listed: wire::Indexers = self.torznab(None, &params).await?;
        if let Some(error) = listed.error {
            return Err(IndexerError::Refused(error));
        }
        Ok(listed
            .indexers
            .into_iter()
            .filter(|indexer| indexer.configured)
            .filter_map(|indexer| match TrackerId::try_from(indexer.id) {
                Ok(id) => Some(Tracker { id, name: indexer.title }),
                Err(error) => {
                    warn!(%error, "a Jackett tracker is left out");
                    None
                },
            })
            .collect())
    }

    async fn fetch(&self, link: &str) -> Result<TorrentSource, IndexerError> {
        if link.starts_with("magnet:") {
            return Ok(TorrentSource::Magnet(link.to_owned()));
        }
        let (url, api_key) = self.server()?;
        if !link.starts_with(&format!("{url}/")) {
            return Err(IndexerError::Refused("the link does not lead to the configured Jackett".into()));
        }
        let mut link = Url::parse(link).map_err(|error| IndexerError::Refused(error.to_string()))?;
        link.query_pairs_mut().append_pair(LINK_KEY, &api_key);
        let response = self.http.get(link).send().await.map_err(|error| IndexerError::Unavailable(error.into()))?;
        let status = response.status();
        debug!(status = status.as_u16(), "Jackett download");
        let location = response.headers().get(LOCATION).and_then(|location| location.to_str().ok());
        match (status, location) {
            (status, Some(magnet)) if status.is_redirection() && magnet.starts_with("magnet:") => {
                Ok(TorrentSource::Magnet(magnet.to_owned()))
            },
            (StatusCode::OK, _) => {
                let bytes = response.bytes().await.map_err(|error| IndexerError::Unavailable(error.into()))?;
                if bytes.first() == Some(&b'd') {
                    Ok(TorrentSource::File(bytes.to_vec()))
                } else {
                    Err(IndexerError::Refused("the tracker sent no torrent".into()))
                }
            },
            (status, _) => Err(IndexerError::Unavailable(format!("HTTP {status}").into())),
        }
    }
}

/// A Torznab item without a link to its torrent.
#[derive(Debug)]
pub(crate) struct NoLink;

impl TryFrom<wire::Item> for Release {
    type Error = NoLink;

    fn try_from(item: wire::Item) -> Result<Self, NoLink> {
        let link = item.download_link().ok_or(NoLink)?;
        let number = |name: &str| item.attr(name).and_then(|value| value.parse::<u32>().ok());
        let seeders = number("seeders");
        let size = item.size.or_else(|| item.attr("size").and_then(|size| size.parse().ok())).unwrap_or(0);
        let published = item.pub_date.as_deref().and_then(|date| jiff::fmt::rfc2822::parse(date).ok());
        Ok(Release {
            tracker: item.jackettindexer.as_ref().map(|indexer| indexer.text.clone()).unwrap_or_default(),
            leechers: number("peers").map(|peers| peers.saturating_sub(seeders.unwrap_or(0))),
            grabs: item.grabs.or_else(|| number("grabs")),
            seeders,
            size,
            published: published.map(|date| date.timestamp()),
            link: without_key(&link),
            details: item.comments,
            title: item.title,
        })
    }
}

/// `link` without the API key Jackett puts in its download links.
fn without_key(link: &str) -> String {
    let Ok(mut url) = Url::parse(link) else { return link.to_owned() };
    if url.scheme() == "magnet" || !url.query_pairs().any(|(name, _)| name == LINK_KEY) {
        return link.to_owned();
    }
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| name != LINK_KEY)
        .map(|(name, value)| (name.into(), value.into()))
        .collect();
    url.query_pairs_mut().clear().extend_pairs(kept);
    url.into()
}

#[cfg(test)]
mod tests {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

    use super::*;

    #[tokio::test]
    async fn a_stalled_server_times_out() {
        let server = MockServer::start().await;
        let stalled = ResponseTemplate::new(200).set_delay(Duration::from_secs(5));
        Mock::given(any()).respond_with(stalled).mount(&server).await;
        let client = JackettClient::with_timeout(
            Live::fixed(JackettSettings { url: Some(server.uri()), api_key: Some(Secret::new("key")) }),
            Duration::from_millis(50),
        );

        let started = Instant::now();
        let error = client.version().await.unwrap_err();

        assert!(matches!(error, IndexerError::Unavailable(_)), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
