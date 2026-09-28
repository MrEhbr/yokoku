//! Shared value types, domain rules and the event contract.

mod artwork;
mod clock;
mod confidence;
mod episode_span;
pub mod events;
mod external;
mod file_status;
mod file_target;
mod folder;
mod id;
mod item;
mod live;
mod movie;
mod secret;
mod series;
mod settings_store;
mod storage_error;
mod string_enum;
mod subtitle;

pub use artwork::{Artwork, ArtworkKind};
pub use clock::Clock;
pub use confidence::Confidence;
pub use episode_span::{EpisodeRef, EpisodeSpan, ParseEpisodeSpanError};
pub use external::{ExternalId, ParseExternalIdError};
pub use file_status::FileStatus;
pub use file_target::FileTarget;
pub use folder::{InvalidFolderName, ItemFolder};
pub use id::{CorrelationId, DownloadId, EpisodeId, ImportId, MediaFileId, MovieId, SeriesId};
pub use item::{ItemId, MediaKind, title_with_year};
pub use live::Live;
pub use movie::{Movie, MovieMetadata, MovieStatus, ReleaseKind, Releases};
pub use secret::Secret;
pub use series::{
    Episode, EpisodeMetadata, MonitorPreset, Numbering, Season, SeasonMetadata, Series, SeriesMetadata, SeriesStatus,
    SourceStatus,
};
pub use settings_store::SettingsStore;
pub use storage_error::StorageError;
pub use string_enum::ParseEnumError;
pub use subtitle::SubtitleTags;
