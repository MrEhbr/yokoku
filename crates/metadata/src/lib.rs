//! Metadata providers: TMDB and TVDB.

mod http;
mod settings;
mod sources;
mod tmdb;
mod tvdb;
mod tvdb_wire;
mod wire;

pub use settings::{MetadataSettings, TmdbSettings, TvdbSettings, UnknownLanguage};
pub use sources::Sources;
pub use tmdb::TmdbClient;
pub use tvdb::TvdbClient;
