use std::sync::Mutex;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{StatusCode, header::HeaderValue};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use yokoku_downloads::{
    DownloadState, DownloadStatus,
    ports::{AddedTorrent, ClientError, DownloadClient, Torrent, TorrentSource},
};

use crate::wire;

const SESSION_HEADER: &str = "X-Transmission-Session-Id";
const LABEL: &str = "yokoku";

/// Talks to Transmission's RPC endpoint, e.g. `http://localhost:9091/transmission/rpc`.
pub struct TransmissionClient {
    http: reqwest::Client,
    url: String,
    credentials: Option<(String, String)>,
    session: Mutex<Option<HeaderValue>>,
}

impl TransmissionClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self { http: reqwest::Client::new(), url: url.into(), credentials: None, session: Mutex::new(None) }
    }

    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.credentials = Some((username.into(), password.into()));
        self
    }

    /// Sends one RPC call, repeating it once with the session id a 409 answer carries.
    async fn call<T: DeserializeOwned>(&self, method: &str, arguments: Value) -> Result<T, ClientError> {
        let body = json!({ "method": method, "arguments": arguments });
        let mut response = self.send(&body).await?;
        if response.status() == StatusCode::CONFLICT {
            let session = response.headers().get(SESSION_HEADER).cloned();
            *self.session.lock().expect("session lock") = session;
            response = self.send(&body).await?;
        }

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
        let mut request = self.http.post(&self.url).json(body);
        if let Some(session) = self.session.lock().expect("session lock").clone() {
            request = request.header(SESSION_HEADER, session);
        }
        if let Some((username, password)) = &self.credentials {
            request = request.basic_auth(username, Some(password));
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
            status: DownloadStatus {
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
