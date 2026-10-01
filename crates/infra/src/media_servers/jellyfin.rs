use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use jiff::Timestamp;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::debug;
use yokoku_core::integrations::ports::{MediaServer, MediaServerError, Played, PlayedItem};
use yokoku_domain::{EpisodeSpan, ExternalId, Live, Secret};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT: Duration = Duration::from_secs(30);
const PAGE_SIZE: usize = 500;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JellyfinSettings {
    /// Server address, e.g. `http://localhost:8096`; rescans are off while unset.
    pub url: Option<String>,
    /// An administrator's API key.
    pub api_key: Option<Secret>,
    /// The user whose played items count as watched; watched sync is off while unset.
    pub user: Option<String>,
}

/// Jellyfin's HTTP API, authenticated with an administrator's API key.
pub struct JellyfinClient {
    http: reqwest::Client,
    settings: Live<JellyfinSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SystemInfo {
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct User {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Items {
    items: Vec<Item>,
    total_record_count: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Item {
    id: String,
    #[serde(rename = "Type")]
    kind: String,
    path: Option<PathBuf>,
    #[serde(default)]
    provider_ids: HashMap<String, String>,
    series_id: Option<String>,
    parent_index_number: Option<u16>,
    index_number: Option<u16>,
    index_number_end: Option<u16>,
    user_data: Option<UserData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UserData {
    last_played_date: Option<Timestamp>,
}

impl Item {
    /// `None` without a path; `series` maps series item ids to their provider ids.
    fn played(self, series: &HashMap<String, Vec<ExternalId>>) -> Option<Played> {
        let item = match self.kind.as_str() {
            "Movie" => Some(PlayedItem::Movie(external_ids(&self.provider_ids))),
            _ => self.span().map(|span| PlayedItem::Episodes {
                series: self.series_id.as_ref().and_then(|id| series.get(id)).cloned().unwrap_or_default(),
                span,
            }),
        };
        Some(Played { path: self.path?, item, at: self.user_data.and_then(|data| data.last_played_date) })
    }

    fn span(&self) -> Option<EpisodeSpan> {
        let first = self.index_number?;
        EpisodeSpan::new(self.parent_index_number?, first, self.index_number_end.unwrap_or(first))
    }
}

/// The TMDB and TVDB ids among Jellyfin's provider ids.
fn external_ids(provider_ids: &HashMap<String, String>) -> Vec<ExternalId> {
    let mut ids: Vec<ExternalId> = provider_ids
        .iter()
        .filter_map(|(provider, id)| match provider.to_ascii_lowercase().as_str() {
            "tmdb" => id.parse().ok().map(ExternalId::Tmdb),
            "tvdb" => id.parse().ok().map(ExternalId::Tvdb),
            _ => None,
        })
        .collect();
    ids.sort_by_key(ToString::to_string);
    ids
}

impl JellyfinClient {
    pub fn new(settings: Live<JellyfinSettings>) -> Self {
        Self::with_timeout(settings, TIMEOUT)
    }

    fn with_timeout(settings: Live<JellyfinSettings>, timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .build()
            .expect("TLS backend initializes");
        Self { http, settings }
    }

    /// Sends `method` to `path` on the configured server; `NotConfigured` while its URL is unset.
    async fn send(&self, method: Method, path: &str) -> Result<reqwest::Response, MediaServerError> {
        let settings = self.settings.current();
        let url = settings.url.as_deref().ok_or(MediaServerError::NotConfigured)?.trim_end_matches('/');
        let api_key = settings.api_key.as_ref().map_or("", Secret::expose);
        let request = self
            .http
            .request(method, format!("{url}{path}"))
            .header("Authorization", format!("MediaBrowser Token=\"{api_key}\""))
            .build()
            .map_err(|error| MediaServerError::Unavailable(error.into()))?;
        let (method, path) = (request.method().clone(), request.url().path().to_owned());
        let started = Instant::now();
        let response = self.http.execute(request).await.map_err(|error| MediaServerError::Unavailable(error.into()))?;
        let elapsed_ms = started.elapsed().as_millis();
        debug!(%method, path, status = response.status().as_u16(), elapsed_ms, "Jellyfin request");
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(MediaServerError::Refused("the API key is missing, wrong or not an administrator's".into()))
            },
            status if !status.is_success() => Err(MediaServerError::Unavailable(format!("HTTP {status}").into())),
            _ => Ok(response),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, MediaServerError> {
        let response = self.send(Method::GET, path).await?;
        response.json().await.map_err(|error| MediaServerError::Unavailable(error.into()))
    }

    /// The id of the configured user; `NotConfigured` while it is unset.
    async fn user_id(&self) -> Result<String, MediaServerError> {
        let name = self.settings.current().user.clone().ok_or(MediaServerError::NotConfigured)?;
        let users: Vec<User> = self.get("/Users").await?;
        users
            .into_iter()
            .find(|user| user.name.to_lowercase() == name.to_lowercase())
            .map(|user| user.id)
            .ok_or_else(|| MediaServerError::Refused(format!("no Jellyfin user is named {name:?}")))
    }

    /// Every library item matching `query`, a page at a time.
    async fn items(&self, query: &str) -> Result<Vec<Item>, MediaServerError> {
        let mut items = Vec::new();
        loop {
            let path = format!(
                "/Items?recursive=true&enableImages=false&{query}&startIndex={}&limit={PAGE_SIZE}",
                items.len()
            );
            let page: Items = self.get(&path).await?;
            let done = page.items.is_empty() || items.len() + page.items.len() >= page.total_record_count;
            items.extend(page.items);
            if done {
                return Ok(items);
            }
        }
    }
}

#[async_trait]
impl MediaServer for JellyfinClient {
    async fn version(&self) -> Result<String, MediaServerError> {
        let info: SystemInfo = self.get("/System/Info").await?;
        Ok(format!("Jellyfin {}", info.version))
    }

    async fn refresh_library(&self) -> Result<(), MediaServerError> {
        self.send(Method::POST, "/Library/Refresh").await.map(drop)
    }

    async fn played(&self) -> Result<Vec<Played>, MediaServerError> {
        let user = self.user_id().await?;
        let played = self
            .items(&format!(
                "userId={user}&isPlayed=true&includeItemTypes=Episode,Movie&fields=Path,ProviderIds&enableUserData=true"
            ))
            .await?;
        let series: HashMap<String, Vec<ExternalId>> = self
            .items(&format!("userId={user}&includeItemTypes=Series&fields=ProviderIds"))
            .await?
            .into_iter()
            .map(|series| (series.id, external_ids(&series.provider_ids)))
            .collect();
        Ok(played.into_iter().filter_map(|item| item.played(&series)).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

    use super::*;

    #[tokio::test]
    async fn a_stalled_server_times_out() {
        let server = MockServer::start().await;
        let stalled = ResponseTemplate::new(204).set_delay(Duration::from_secs(5));
        Mock::given(any()).respond_with(stalled).mount(&server).await;
        let client = JellyfinClient::with_timeout(
            Live::fixed(JellyfinSettings { url: Some(server.uri()), api_key: Some(Secret::new("secret")), user: None }),
            Duration::from_millis(50),
        );

        let started = Instant::now();
        let error = client.refresh_library().await.unwrap_err();

        assert!(matches!(error, MediaServerError::Unavailable(_)), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
