//! Shared value types, domain rules and the event contract.

mod disk;
pub mod events;
mod id;
mod item;
mod live;
mod matching;
mod movie;
pub mod naming;
mod ports;
mod secret;
mod series;
mod string_enum;

pub use disk::DiskSpace;
pub use id::{CorrelationId, DownloadId, EpisodeId, ImportId, MediaFileId, MovieId, SeriesId};
pub use item::{
    Artwork, ArtworkKind, Description, ExternalId, FileStatus, InvalidFolderName, ItemFolder, ItemId, ItemName,
    MediaKind, ParseExternalIdError,
};
pub use live::Live;
pub use matching::{Confidence, EpisodeRef, EpisodeSpan, FileTarget, ParseEpisodeSpanError, SubtitleTags};
pub use movie::{Movie, MovieMetadata, MovieStatus, ReleaseKind, Releases};
pub use ports::{Clock, SettingsStore, StorageError};
pub use secret::Secret;
pub use series::{
    Episode, EpisodeMetadata, MonitorPreset, Numbering, Season, SeasonMetadata, Series, SeriesMetadata, SeriesStatus,
    SourceStatus,
};
pub use string_enum::ParseEnumError;
