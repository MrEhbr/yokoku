use std::{fmt, path::PathBuf, time::Duration};

use jiff::Timestamp;
use yokoku_domain::{
    Confidence, DownloadId, EpisodeSpan, FileTarget, ImportId, MediaFileId, MovieId, SeriesId, SubtitleTags,
    events::LinkedFile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    Series,
    Movies,
}

yokoku_domain::string_enum!(RootKind, "root folder kind" {
    Series => "series",
    Movies => "movies",
});

/// A folder holding the library's series or movies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootFolder {
    pub kind: RootKind,
    pub path: PathBuf,
}

/// A video file in a root folder, linked to what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaFile {
    pub id: MediaFileId,
    pub path: PathBuf,
    pub size: u64,
    pub target: FileTarget,
    pub added_at: Timestamp,
}

impl MediaFile {
    pub(crate) fn linked(&self) -> LinkedFile {
        LinkedFile { file: self.id, path: self.path.clone(), target: self.target }
    }
}

/// Files waiting for the user to confirm what they hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub id: ImportId,
    /// The folder or file the rows come from.
    pub source: PathBuf,
    /// The download the files come from; files found by a scan have none and stay where they are.
    pub download: Option<DownloadId>,
    pub status: ImportStatus,
    /// Why the last attempt failed.
    pub error: Option<String>,
    /// Ordered by path.
    pub rows: Vec<ImportRow>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStatus {
    NeedsReview,
    /// Waiting to be carried out.
    Approved,
    Importing,
    Done,
    Failed,
}

yokoku_domain::string_enum!(ImportStatus, "import status" {
    NeedsReview => "needs_review",
    Approved => "approved",
    Importing => "importing",
    Done => "done",
    Failed => "failed",
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRow {
    pub path: PathBuf,
    pub size: u64,
    pub matched: RowMatch,
    pub confidence: Confidence,
    /// Unchecked: left where it is.
    pub skipped: bool,
    pub resolution: Resolution,
}

impl ImportRow {
    /// The complete match, whether or not the library has it.
    pub fn target(&self) -> Option<FileTarget> {
        self.matched.target()
    }

    /// Marks the row matched by the user: checked, certain, and settling no conflict yet.
    pub(crate) fn confirm(&mut self) {
        self.confidence = Confidence::Certain;
        self.skipped = false;
        self.resolution = Resolution::Unresolved;
    }
}

/// What a row holds as far as it is known; its season and episodes need not be in the series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowMatch {
    #[default]
    None,
    Series {
        series: SeriesId,
        season: Option<u16>,
        episodes: Option<Episodes>,
    },
    Movie(MovieId),
}

/// Consecutive episode numbers of an unnamed season.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Episodes {
    pub first: u16,
    pub last: u16,
}

impl Episodes {
    /// The first and last of sorted episode numbers; `None` when there are none.
    pub fn spanning(sorted: &[u16]) -> Option<Self> {
        Some(Self { first: *sorted.first()?, last: *sorted.last()? })
    }
}

impl RowMatch {
    pub fn target(&self) -> Option<FileTarget> {
        match *self {
            Self::None => None,
            Self::Series { series, season: Some(season), episodes: Some(Episodes { first, last }) } => {
                Some(FileTarget::Episodes { series, span: EpisodeSpan::new(season, first, last)? })
            },
            Self::Series { .. } => None,
            Self::Movie(movie) => Some(FileTarget::Movie(movie)),
        }
    }
}

impl From<FileTarget> for RowMatch {
    fn from(target: FileTarget) -> Self {
        match target {
            FileTarget::Episodes { series, span } => Self::Series {
                series,
                season: Some(span.season()),
                episodes: Some(Episodes { first: span.first(), last: span.last() }),
            },
            FileTarget::Movie(movie) => Self::Movie(movie),
        }
    }
}

/// How a row settles a library file, or another row, that holds its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resolution {
    /// Either is a conflict the user resolves.
    #[default]
    Unresolved,
    /// Replaces the library file; another row is still a conflict.
    Replace,
    /// Imported beside both; a download's file takes a numbered name where its own is taken.
    KeepBoth,
}

yokoku_domain::string_enum!(Resolution, "resolution" {
    Unresolved => "unresolved",
    Replace => "replace",
    KeepBoth => "keep-both",
});

/// Streams a probe read from a video file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub video: Option<VideoStream>,
    /// In file order.
    pub audio: Vec<AudioStream>,
    /// Subtitles inside the file, in file order.
    pub subtitles: Vec<SubtitleStream>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStream {
    pub codec: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStream {
    pub codec: String,
    /// ISO 639-2, like `eng`.
    pub language: Option<String>,
    pub channels: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleStream {
    pub codec: String,
    /// ISO 639-2, like `eng`.
    pub language: Option<String>,
    pub forced: bool,
}

/// E.g. `1920x1080 h264`.
impl fmt::Display for VideoStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{} {}", self.width, self.height, self.codec)
    }
}

/// E.g. `eng aac 5.1`.
impl fmt::Display for AudioStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} ", self.language.as_deref().unwrap_or("unknown"), self.codec)?;
        match self.channels {
            1 => f.write_str("mono"),
            2 => f.write_str("stereo"),
            6 => f.write_str("5.1"),
            8 => f.write_str("7.1"),
            count => write!(f, "{count} channels"),
        }
    }
}

/// The language, e.g. `eng (forced)`.
impl fmt::Display for SubtitleStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SubtitleTags { language: self.language.clone(), sdh: false, forced: self.forced }.fmt(f)
    }
}
