//! Shared value types and domain rules.

mod external;
mod file_status;
mod id;
mod movie;
mod series;

pub use external::{ExternalId, ParseExternalIdError};
pub use file_status::FileStatus;
pub use id::{EpisodeId, MovieId, SeriesId};
pub use movie::{Movie, MovieMetadata, MovieStatus, ReleaseKind, Releases};
pub use series::{
    Episode, EpisodeMetadata, EpisodeRef, MonitorPreset, Numbering, Season, SeasonMetadata, Series, SeriesMetadata,
    SeriesStatus, SourceStatus,
};
