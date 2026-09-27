//! Shared value types, domain rules and the event contract.

mod clock;
mod confidence;
pub mod events;
mod external;
mod file_status;
mod file_target;
mod folder;
mod id;
mod item;
mod movie;
mod series;
mod storage_error;
mod string_enum;
mod subtitle;

pub use clock::Clock;
pub use confidence::Confidence;
pub use external::{ExternalId, ParseExternalIdError};
pub use file_status::FileStatus;
pub use file_target::FileTarget;
pub use folder::{InvalidFolderName, ItemFolder};
pub use id::{CorrelationId, DownloadId, EpisodeId, ImportId, MediaFileId, MovieId, SeriesId};
pub use item::{ItemId, MediaKind, title_with_year};
pub use movie::{Movie, MovieMetadata, MovieStatus, ReleaseKind, Releases};
pub use series::{
    Episode, EpisodeMetadata, EpisodeRef, EpisodeSpan, MonitorPreset, Numbering, ParseEpisodeSpanError, Season,
    SeasonMetadata, Series, SeriesMetadata, SeriesStatus, SourceStatus,
};
pub use storage_error::StorageError;
pub use string_enum::ParseEnumError;
pub use subtitle::SubtitleTags;
