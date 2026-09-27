//! SQLite persistence: migrations, repositories and the event store.

mod catalog;
mod codec;
mod database;
mod download_repo;
mod error;
mod event_log;
mod media_files;
mod media_info;
mod media_repo;
mod movie_repo;
mod rescan_store;
mod series_repo;
mod settings_store;

pub use database::Database;
pub use error::DbError;
pub use event_log::SqliteEventLog;
