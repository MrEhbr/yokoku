//! Metadata providers: TMDB and TVDB.

mod http;
mod tmdb;
mod wire;

pub use tmdb::TmdbClient;
