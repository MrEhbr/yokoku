//! Catalog, monitoring, metadata refresh and calendar queries.

mod calendar;
mod error;
mod files;
mod library;
mod listing;
mod metadata;
pub mod ports;
mod retry;
mod snapshot;

pub use calendar::{
    Calendar, CalendarEntry, CalendarRelease, Missing, MissingEpisode, MissingMovie, MissingSeries, month_of, week_of,
};
pub use error::LibraryError;
pub use files::FileTracker;
pub use library::Library;
pub use listing::{LibraryEntry, LibraryFilter, LibrarySort, LibraryStatus};
pub use metadata::{MetadataService, RefreshFailure, RefreshReport, SearchHit};
