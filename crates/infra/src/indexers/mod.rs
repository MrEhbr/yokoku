//! Indexer adapters: Jackett, through its Torznab API.

mod jackett;
mod torznab_wire;

pub use jackett::{JackettClient, JackettSettings};
