//! Indexer adapters for Jackett and Prowlarr.

mod jackett;
mod prowlarr;
mod selected;
mod torznab_wire;

pub use jackett::{JackettClient, JackettSettings};
pub use prowlarr::{ProwlarrClient, ProwlarrSettings};
pub use selected::{IndexerProvider, SelectedIndexer};
