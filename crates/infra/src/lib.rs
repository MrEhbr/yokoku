//! Adapters: SQLite persistence, metadata providers, download clients, media servers and the local
//! system.

pub mod db;
pub mod download_clients;
pub mod media_servers;
pub mod metadata;
pub mod system;
mod tls;
