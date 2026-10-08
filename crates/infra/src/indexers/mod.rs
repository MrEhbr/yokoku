//! Indexer adapters: Jackett, through its Torznab API.

mod combined;
mod feeds;
mod jackett;
mod torznab;
mod torznab_wire;

pub use combined::CombinedIndexer;
pub use feeds::{TorznabFeed, TorznabSettings};
pub use jackett::{JackettClient, JackettSettings};
pub use torznab::TorznabClient;
