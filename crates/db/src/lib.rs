//! SQLite persistence: migrations, repositories and the event store.

mod codec;
mod database;
mod download_repo;
mod error;
mod event_log;
mod media_repo;
mod movie_repo;
mod rescan_store;
mod series_repo;

pub use database::Database;
pub use error::DbError;
pub use event_log::SqliteEventLog;
