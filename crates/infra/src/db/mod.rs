//! SQLite persistence: migrations and repositories.

mod catalog;
mod codec;
mod database;
mod downloads;
mod error;
mod events;
mod media;
mod media_info;
mod movies;
mod ratings;
mod rescans;
mod series;
mod settings;
mod watched;

pub use database::Database;
pub use error::DbError;
