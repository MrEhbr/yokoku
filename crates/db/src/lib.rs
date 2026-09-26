//! SQLite persistence: migrations, repositories and the event store.

mod database;
mod error;
mod event_log;

pub use database::Database;
pub use error::DbError;
pub use event_log::SqliteEventLog;
