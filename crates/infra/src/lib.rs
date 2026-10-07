//! Adapters: SQLite persistence, metadata providers, ratings, download clients, indexers, media
//! servers and the local system.

pub mod db;
pub mod download_clients;
pub mod indexers;
pub mod media_servers;
pub mod metadata;
pub mod ratings;
pub mod system;
mod tls;
