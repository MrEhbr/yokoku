use std::time::{Duration, Instant};

use async_trait::async_trait;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::debug;
use yokoku_core::integrations::ports::{MediaServer, MediaServerError};
use yokoku_domain::{Live, Secret};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JellyfinSettings {
    /// Server address, e.g. `http://localhost:8096`; rescans are off while unset.
    pub url: Option<String>,
    /// An administrator's API key.
    pub api_key: Option<Secret>,
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
            .map_err(unavailable)?;
        let (method, path) = (request.method().clone(), request.url().path().to_owned());
        let started = Instant::now();
        let response = self.http.execute(request).await.map_err(unavailable)?;
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
}

#[async_trait]
impl MediaServer for JellyfinClient {
    async fn version(&self) -> Result<String, MediaServerError> {
        let response = self.send(Method::GET, "/System/Info").await?;
        let info: SystemInfo = response.json().await.map_err(unavailable)?;
        Ok(format!("Jellyfin {}", info.version))
    }

    async fn refresh_library(&self) -> Result<(), MediaServerError> {
        self.send(Method::POST, "/Library/Refresh").await.map(drop)
    }
}

fn unavailable(error: reqwest::Error) -> MediaServerError {
    MediaServerError::Unavailable(Box::new(error))
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
            Live::fixed(JellyfinSettings { url: Some(server.uri()), api_key: Some(Secret::new("secret")) }),
            Duration::from_millis(50),
        );

        let started = Instant::now();
        let error = client.refresh_library().await.unwrap_err();

        assert!(matches!(error, MediaServerError::Unavailable(_)), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
