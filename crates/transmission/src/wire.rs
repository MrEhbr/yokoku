use serde::{Deserialize, de::DeserializeOwned};

#[derive(Deserialize)]
#[serde(bound = "T: DeserializeOwned")]
pub(crate) struct Response<T> {
    pub result: String,
    pub arguments: Option<T>,
}

#[derive(Deserialize)]
pub(crate) struct Session {
    pub version: String,
}

#[derive(Deserialize)]
pub(crate) struct Added {
    #[serde(rename = "torrent-added")]
    pub added: Option<AddedTorrent>,
    #[serde(rename = "torrent-duplicate")]
    pub duplicate: Option<AddedTorrent>,
}

#[derive(Deserialize)]
pub(crate) struct AddedTorrent {
    #[serde(rename = "hashString")]
    pub hash: String,
    pub name: String,
}

#[derive(Deserialize)]
pub(crate) struct Torrents {
    pub torrents: Vec<Torrent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Torrent {
    pub hash_string: String,
    pub name: String,
    /// 0 stopped, 1 queued to check, 2 checking, 3 queued to download, 4 downloading,
    /// 5 queued to seed, 6 seeding.
    pub status: i64,
    pub size_when_done: u64,
    pub left_until_done: u64,
    pub rate_download: u64,
    /// Seconds; -1 when not available, -2 when unknown.
    pub eta: i64,
    pub download_dir: String,
    /// 0 when there is no error.
    pub error: i64,
    pub error_string: String,
    pub metadata_percent_complete: f64,
}

pub(crate) const TORRENT_FIELDS: [&str; 11] = [
    "hashString",
    "name",
    "status",
    "sizeWhenDone",
    "leftUntilDone",
    "rateDownload",
    "eta",
    "downloadDir",
    "error",
    "errorString",
    "metadataPercentComplete",
];
