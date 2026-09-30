//! Metadata providers: TMDB and TVDB, and their artwork.

mod artwork;
mod dates;
mod http;
mod settings;
mod sources;
mod tmdb;
mod tmdb_wire;
mod tvdb;
mod tvdb_wire;

pub use artwork::ArtworkFetcher;
pub use settings::{MetadataSettings, TmdbSettings, TvdbSettings, UnknownLanguage};
pub use sources::Sources;
pub use tmdb::TmdbClient;
pub use tvdb::TvdbClient;
