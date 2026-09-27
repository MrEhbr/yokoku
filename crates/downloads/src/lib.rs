//! Torrents, download client sync and seeding cleanup.

mod downloads;
mod error;
mod model;
pub mod ports;

pub use downloads::{DownloadOptions, Downloads, PickUp, SyncReport};
pub use error::DownloadError;
pub use model::{Download, DownloadState, TorrentStatus};
