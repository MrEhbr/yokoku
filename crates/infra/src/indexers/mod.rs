//! Indexer adapters for Jackett and Prowlarr.

mod combined;
mod jackett;
mod prowlarr;
mod torznab_wire;

pub use combined::CombinedIndexer;
pub use jackett::{JackettClient, JackettSettings};
pub use prowlarr::{ProwlarrClient, ProwlarrSettings};
