use std::{collections::HashSet, fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{DownloadId, EpisodeSpan, FileTarget, ImportId, ItemId, MediaFileId, MovieId, SeriesId};

macro_rules! events {
    ($($name:ident),* $(,)?) => {
        /// Stored events never change meaning; a breaking change adds a new variant.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(tag = "type")]
        pub enum Event {
            $($name($name)),*
        }

        impl Event {
            /// The variant name, as stored in `type`.
            pub fn name(&self) -> &'static str {
                match self {
                    $(Self::$name(_) => stringify!($name)),*
                }
            }
        }

        $(
            impl From<$name> for Event {
                fn from(event: $name) -> Self {
                    Self::$name(event)
                }
            }

            impl EventKind for $name {
                fn from_event(event: &Event) -> Option<&Self> {
                    match event {
                        Event::$name(inner) => Some(inner),
                        _ => None,
                    }
                }
            }
        )*
    };
}

/// One type of event; `Event::get` finds it in an `Event`.
pub trait EventKind: Into<Event> + Send + Sync + 'static {
    fn from_event(event: &Event) -> Option<&Self>;
}

events!(
    SeriesAdded,
    MovieAdded,
    SeriesRemoved,
    MovieRemoved,
    FilesFound,
    FilesImported,
    FileDeleted,
    FileRenamed,
    ImportNeedsReview,
    ImportFailed,
    TorrentAdded,
    DownloadCompleted,
    TorrentRemoved,
    EpisodesRenumbered,
    SettingsChanged,
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesAdded {
    pub series: SeriesId,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovieAdded {
    pub movie: MovieId,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesRemoved {
    pub series: SeriesId,
    pub title: String,
    pub delete_files: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovieRemoved {
    pub movie: MovieId,
    pub title: String,
    pub delete_files: bool,
}

/// A scan linked files already in a root folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesFound {
    pub files: Vec<LinkedFile>,
}

/// An approved import placed files in the library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesImported {
    pub import: ImportId,
    /// `None` for scanned files, and in events stored before imports carried it.
    #[serde(default)]
    pub download: Option<DownloadId>,
    pub files: Vec<LinkedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDeleted {
    pub file: MediaFileId,
    pub path: PathBuf,
    pub target: FileTarget,
    pub reason: DeleteReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRenamed {
    pub file: MediaFileId,
    pub from: PathBuf,
    pub to: PathBuf,
    /// `None` in events stored before renames carried it.
    #[serde(default)]
    pub target: Option<FileTarget>,
}

/// A metadata refresh gave episodes holding files new numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodesRenumbered {
    pub series: SeriesId,
    pub files: Vec<RenumberedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenumberedFile {
    pub file: MediaFileId,
    /// The episodes now holding the file; `None` when they no longer form one span, and the
    /// episodes no longer hold it.
    pub span: Option<EpisodeSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportNeedsReview {
    pub import: ImportId,
    pub source: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFailed {
    pub import: ImportId,
    pub source: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorrentAdded {
    pub download: DownloadId,
    pub name: String,
    pub item: Option<ItemId>,
}

/// Emitted once per download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadCompleted {
    pub download: DownloadId,
    pub name: String,
    pub content_path: PathBuf,
    pub item: Option<ItemId>,
}

/// Removed from the download client, with its data, after its import once seeding finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorrentRemoved {
    pub download: DownloadId,
    pub name: String,
    pub item: Option<ItemId>,
}

/// A stored setting was set or unset; the value is left out, since it may be a secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsChanged {
    pub key: String,
}

impl Event {
    /// `None` when the event is of another type.
    pub fn get<E: EventKind>(&self) -> Option<&E> {
        E::from_event(self)
    }

    /// The series and movies the event concerns, each once.
    pub fn items(&self) -> Vec<ItemId> {
        let mut items: Vec<ItemId> = match self {
            Self::SeriesAdded(SeriesAdded { series, .. })
            | Self::SeriesRemoved(SeriesRemoved { series, .. })
            | Self::EpisodesRenumbered(EpisodesRenumbered { series, .. }) => {
                vec![ItemId::Series(*series)]
            },
            Self::MovieAdded(MovieAdded { movie, .. }) | Self::MovieRemoved(MovieRemoved { movie, .. }) => {
                vec![ItemId::Movie(*movie)]
            },
            Self::FilesFound(FilesFound { files }) | Self::FilesImported(FilesImported { files, .. }) => {
                files.iter().map(|file| file.target.item()).collect()
            },
            Self::FileDeleted(FileDeleted { target, .. }) => vec![target.item()],
            Self::FileRenamed(FileRenamed { target, .. }) => target.iter().map(FileTarget::item).collect(),
            Self::TorrentAdded(TorrentAdded { item, .. })
            | Self::DownloadCompleted(DownloadCompleted { item, .. })
            | Self::TorrentRemoved(TorrentRemoved { item, .. }) => item.iter().copied().collect(),
            Self::ImportNeedsReview(_) | Self::ImportFailed(_) | Self::SettingsChanged(_) => Vec::new(),
        };
        let mut seen = HashSet::new();
        items.retain(|item| seen.insert(*item));
        items
    }
}

/// One line, followed by one line per file where the event holds several.
impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let linked = |f: &mut fmt::Formatter<'_>, verb: &str, files: &[LinkedFile]| match files {
            [file] => write!(f, "{verb} {}", file.path.display()),
            files => {
                write!(f, "{verb} {} files", files.len())?;
                files.iter().try_for_each(|file| write!(f, "\n{}", file.path.display()))
            },
        };
        let with_files = |delete_files: bool| if delete_files { " and its files" } else { "" };
        match self {
            Self::SeriesAdded(SeriesAdded { title, .. }) => write!(f, "Added series {title}"),
            Self::MovieAdded(MovieAdded { title, .. }) => write!(f, "Added movie {title}"),
            Self::SeriesRemoved(SeriesRemoved { title, delete_files, .. }) => {
                write!(f, "Removed series {title}{}", with_files(*delete_files))
            },
            Self::MovieRemoved(MovieRemoved { title, delete_files, .. }) => {
                write!(f, "Removed movie {title}{}", with_files(*delete_files))
            },
            Self::FilesFound(FilesFound { files }) => linked(f, "Found", files),
            Self::FilesImported(FilesImported { files, .. }) => linked(f, "Imported", files),
            Self::FileDeleted(FileDeleted { path, reason, .. }) => {
                let reason = match reason {
                    DeleteReason::External => "gone from disk",
                    DeleteReason::Replaced => "replaced by an import",
                    DeleteReason::User => "by request",
                    DeleteReason::ItemRemoved => "its item was removed",
                };
                write!(f, "Deleted {} ({reason})", path.display())
            },
            Self::FileRenamed(FileRenamed { from, to, .. }) => {
                write!(f, "Renamed {}\n-> {}", from.display(), to.display())
            },
            Self::EpisodesRenumbered(EpisodesRenumbered { files, .. }) => match files.len() {
                1 => write!(f, "Renumbered the episodes of 1 file"),
                count => write!(f, "Renumbered the episodes of {count} files"),
            },
            Self::ImportNeedsReview(ImportNeedsReview { source, .. }) => {
                write!(f, "Import of {} needs review", source.display())
            },
            Self::ImportFailed(ImportFailed { source, reason, .. }) => {
                write!(f, "Import of {} failed: {reason}", source.display())
            },
            Self::TorrentAdded(TorrentAdded { name, .. }) => write!(f, "Added torrent {name}"),
            Self::DownloadCompleted(DownloadCompleted { name, .. }) => write!(f, "Finished downloading {name}"),
            Self::TorrentRemoved(TorrentRemoved { name, .. }) => write!(f, "Removed torrent {name} after seeding"),
            Self::SettingsChanged(SettingsChanged { key }) => write!(f, "Changed setting {key}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedFile {
    pub file: MediaFileId,
    pub path: PathBuf,
    pub target: FileTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteReason {
    /// The file disappeared outside the app.
    External,
    /// An imported file took its place.
    Replaced,
    /// The user deleted it.
    User,
    /// Its series or movie was removed with its files.
    ItemRemoved,
}
