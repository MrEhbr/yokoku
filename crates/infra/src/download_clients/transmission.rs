use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{StatusCode, header::HeaderValue};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tracing::debug;
use yokoku_domain::{Live, Secret};
use yokoku_downloads::{
    DownloadState, TorrentStatus,
    ports::{AddedTorrent, ClientError, DownloadClient, LABEL, Torrent, TorrentSource},
};

use crate::download_clients::transmission_wire as wire;

const SESSION_HEADER: &str = "X-Transmission-Session-Id";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransmissionSettings {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<Secret>,
}

impl Default for TransmissionSettings {
    fn default() -> Self {
        Self { url: "http://localhost:9091/transmission/rpc".into(), username: None, password: None }
    }
}

/// Talks to Transmission's RPC endpoint, e.g. `http://localhost:9091/transmission/rpc`.
pub struct TransmissionClient {
    http: reqwest::Client,
    settings: Live<TransmissionSettings>,
    session: Mutex<Option<HeaderValue>>,
}

impl TransmissionClient {
    pub fn new(settings: Live<TransmissionSettings>) -> Self {
        Self::with_timeout(settings, TIMEOUT)
    }

    fn with_timeout(settings: Live<TransmissionSettings>, timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .build()
            .expect("TLS backend initializes");
        Self { http, settings, session: Mutex::new(None) }
    }

    /// Sends one RPC call, repeating it once with the session id a 409 answer carries.
    async fn call<T: DeserializeOwned>(&self, method: &str, arguments: Value) -> Result<T, ClientError> {
        let started = Instant::now();
        let body = json!({ "method": method, "arguments": arguments });
        let mut response = self.send(&body).await?;
        if response.status() == StatusCode::CONFLICT {
            debug!("renewing the Transmission session id");
            let session = response.headers().get(SESSION_HEADER).cloned();
            *self.session.lock().expect("session lock") = session;
            response = self.send(&body).await?;
        }
        debug!(
            method,
            status = response.status().as_u16(),
            elapsed_ms = started.elapsed().as_millis(),
            "Transmission call"
        );

        match response.status() {
            StatusCode::UNAUTHORIZED => return Err(ClientError::Refused("wrong username or password".into())),
            status if !status.is_success() => return Err(ClientError::Unavailable(format!("HTTP {status}").into())),
            _ => {},
        }
        let response: wire::Response<T> = response.json().await.map_err(unavailable)?;
        match (response.result.as_str(), response.arguments) {
            ("success", Some(arguments)) => Ok(arguments),
            ("success", None) => Err(ClientError::Unavailable("response without arguments".into())),
            (result, _) => Err(ClientError::Refused(result.to_owned())),
        }
    }

    async fn send(&self, body: &Value) -> Result<reqwest::Response, ClientError> {
        let settings = self.settings.current();
        let mut request = self.http.post(&settings.url).json(body);
        if let Some(session) = self.session.lock().expect("session lock").clone() {
            request = request.header(SESSION_HEADER, session);
        }
        if let Some(username) = &settings.username {
            request = request.basic_auth(username, Some(settings.password.as_ref().map_or("", Secret::expose)));
        }
        request.send().await.map_err(unavailable)
    }
}

#[async_trait]
impl DownloadClient for TransmissionClient {
    async fn version(&self) -> Result<String, ClientError> {
        let session: wire::Session = self.call("session-get", json!({ "fields": ["version"] })).await?;
        Ok(format!("Transmission {}", session.version))
    }

    async fn add(&self, torrent: &TorrentSource) -> Result<AddedTorrent, ClientError> {
        let arguments = match torrent {
            TorrentSource::Magnet(link) => json!({ "filename": link, "labels": [LABEL] }),
            TorrentSource::File(bytes) => json!({ "metainfo": STANDARD.encode(bytes), "labels": [LABEL] }),
        };
        let added: wire::Added = self.call("torrent-add", arguments).await?;
        let torrent = added
            .added
            .or(added.duplicate)
            .ok_or_else(|| ClientError::Unavailable("torrent-add answered without a torrent".into()))?;
        Ok(AddedTorrent { hash: torrent.hash.to_ascii_lowercase(), name: torrent.name })
    }

    async fn torrents(&self, hashes: &[String]) -> Result<Vec<Torrent>, ClientError> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }
        let torrents: wire::Torrents =
            self.call("torrent-get", json!({ "fields": wire::TORRENT_FIELDS, "ids": hashes })).await?;
        Ok(torrents.torrents.into_iter().map(Torrent::from).collect())
    }

    async fn all_torrents(&self) -> Result<Vec<Torrent>, ClientError> {
        let torrents: wire::Torrents = self.call("torrent-get", json!({ "fields": wire::TORRENT_FIELDS })).await?;
        Ok(torrents.torrents.into_iter().map(Torrent::from).collect())
    }

    async fn remove(&self, hash: &str, delete_data: bool) -> Result<(), ClientError> {
        let arguments = json!({ "ids": [hash], "delete-local-data": delete_data });
        self.call::<Value>("torrent-remove", arguments).await.map(drop)
    }
}

impl From<wire::Torrent> for Torrent {
    fn from(torrent: wire::Torrent) -> Self {
        let state = match torrent.status {
            0 => DownloadState::Stopped,
            1 | 2 => DownloadState::Checking,
            3 => DownloadState::Queued,
            4 => DownloadState::Downloading,
            _ => DownloadState::Seeding,
        };
        let has_metadata = torrent.metadata_percent_complete >= 1.0;
        let complete = has_metadata
            && torrent.size_when_done > 0
            && torrent.left_until_done == 0
            && state != DownloadState::Checking;
        Torrent {
            hash: torrent.hash_string.to_ascii_lowercase(),
            name: torrent.name,
            status: TorrentStatus {
                state,
                size: torrent.size_when_done,
                done: torrent.size_when_done.saturating_sub(torrent.left_until_done),
                download_rate: torrent.rate_download,
                eta: u64::try_from(torrent.eta).ok(),
                download_dir: torrent.download_dir.into(),
                error: (torrent.error != 0).then_some(torrent.error_string),
            },
            complete,
            seeding_done: torrent.is_finished,
            labels: torrent.labels,
        }
    }
}

fn unavailable(error: reqwest::Error) -> ClientError {
    ClientError::Unavailable(Box::new(error))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

    use super::*;

    #[tokio::test]
    async fn a_stalled_server_times_out() {
        let server = MockServer::start().await;
        let stalled = ResponseTemplate::new(200).set_delay(Duration::from_secs(5));
        Mock::given(any()).respond_with(stalled).mount(&server).await;
        let client = TransmissionClient::with_timeout(
            Live::fixed(TransmissionSettings { url: server.uri(), ..TransmissionSettings::default() }),
            Duration::from_millis(50),
        );

        let started = Instant::now();
        let error = client.version().await.unwrap_err();

        assert!(matches!(error, ClientError::Unavailable(_)), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
