//! Shared value types and domain rules.

mod clock;
mod confidence;
mod external;
mod file_status;
mod file_target;
mod id;
mod movie;
mod series;
mod subtitle;

pub use clock::Clock;
pub use confidence::Confidence;
pub use external::{ExternalId, ParseExternalIdError};
pub use file_status::FileStatus;
pub use file_target::FileTarget;
pub use id::{EpisodeId, ImportId, MediaFileId, MovieId, SeriesId};
pub use movie::{Movie, MovieMetadata, MovieStatus, ReleaseKind, Releases};
pub use series::{
    Episode, EpisodeMetadata, EpisodeRef, EpisodeSpan, MonitorPreset, Numbering, ParseEpisodeSpanError, Season,
    SeasonMetadata, Series, SeriesMetadata, SeriesStatus, SourceStatus,
};
pub use subtitle::SubtitleTags;
