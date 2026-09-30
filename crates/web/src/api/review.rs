//! Matching the files detection was unsure about, then importing them (FR-4.11, 4.12, 8.3).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ImportId, ItemId, MovieId, SeriesId};

#[cfg(feature = "server")]
use crate::api::{Dep, Library, Reviewer};

/// An import waiting for review; rows are numbered from 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub id: ImportId,
    pub source: String,
    /// From a download, so a row may replace a library file; a scan's files stay where they are.
    pub from_download: bool,
    pub rows: Vec<ReviewFile>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewFile {
    pub row: usize,
    /// Relative to the import's source.
    pub path: String,
    /// Bytes.
    pub size: u64,
    pub target: Option<Target>,
    pub confidence: Confidence,
    pub skipped: bool,
    /// Replaces the library file that holds its target.
    pub replace: bool,
    /// Why it cannot be imported as matched, like "already has a file".
    pub conflicts: Vec<Conflict>,
}

/// What a file is matched to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Match {
    Episodes { series: SeriesId, season: u16, first: u16, last: u16 },
    Movie { id: MovieId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub matched: Match,
    pub item: ItemId,
    /// `Frieren · S01E02`, or the movie's title.
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Certain,
    Guess,
    Unknown,
}

/// Why a matched file cannot be imported as it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Conflict {
    /// Another file in the import has the same episode or movie.
    SharedTarget,
    /// The episode or movie already has a library file.
    AlreadyHasFile,
}

impl Conflict {
    pub fn label(self) -> &'static str {
        match self {
            Self::SharedTarget => "Another file has the same match",
            Self::AlreadyHasFile => "It already has a file",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Imported {
    /// A scan's files, linked where they are.
    Linked(usize),
    /// A download's files, which the import job places.
    Queued,
}

impl Match {
    pub fn item(self) -> ItemId {
        match self {
            Self::Episodes { series, .. } => ItemId::Series(series),
            Self::Movie { id } => ItemId::Movie(id),
        }
    }
}

/// `None` once the import no longer waits for review.
#[post("/api/review", reviewer: Dep<Reviewer>, library: Dep<Library>)]
pub async fn review(import: ImportId) -> Result<Option<Review>, ServerFnError> {
    server::review(&reviewer, &library, import).await
}

#[post("/api/review/match", reviewer: Dep<Reviewer>)]
pub async fn match_file(import: ImportId, row: usize, target: Match) -> Result<(), ServerFnError> {
    server::match_file(&reviewer, import, row, target).await
}

/// The file is left where it is and not offered again.
#[post("/api/review/skip", reviewer: Dep<Reviewer>)]
pub async fn skip_file(import: ImportId, row: usize) -> Result<(), ServerFnError> {
    reviewer.skip_row(import, row).await.map_err(server::failure)
}

/// The downloaded file replaces the library file of its episode or movie.
#[post("/api/review/replace", reviewer: Dep<Reviewer>)]
pub async fn replace_file(import: ImportId, row: usize) -> Result<(), ServerFnError> {
    reviewer.replace_row(import, row).await.map_err(server::failure)
}

/// Every row not skipped needs a match free of conflicts.
#[post("/api/review/approve", reviewer: Dep<Reviewer>)]
pub async fn approve(import: ImportId) -> Result<Imported, ServerFnError> {
    server::approve(&reviewer, import).await
}

#[cfg(feature = "server")]
mod server {
    use std::collections::HashMap;

    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_domain::{EpisodeSpan, FileTarget, ImportId, ItemId};
    use yokoku_library::{LibraryFilter, LibrarySort};
    use yokoku_media::{Approval, MediaError};

    use super::{Confidence, Conflict, Imported, Library, Match, Review, ReviewFile, Reviewer, Target};

    pub(super) async fn review(
        reviewer: &Reviewer,
        library: &Library,
        import: ImportId,
    ) -> Result<Option<Review>, ServerFnError> {
        let review = match reviewer.get(import).await {
            Ok(review) => review,
            Err(MediaError::ImportNotFound(_) | MediaError::NotInReview(_)) => return Ok(None),
            Err(error) => return Err(failure(error)),
        };
        let items = library.list(LibraryFilter::default(), LibrarySort::Title).await.map_err(|error| {
            error!(%error, "listing the library failed");
            ServerFnError::new("The library could not be loaded")
        })?;
        let titles: HashMap<ItemId, String> = items.into_iter().map(|entry| (entry.id, entry.title)).collect();
        let rows = (1..)
            .zip(review.rows)
            .map(|(number, reviewed)| {
                let row = reviewed.row;
                let target = row.target.map(|target| {
                    let matched = Match::from(target);
                    let title = titles.get(&matched.item()).cloned().unwrap_or_default();
                    let label = match target {
                        FileTarget::Episodes { span, .. } => format!("{title} · {span}"),
                        FileTarget::Movie(_) => title,
                    };
                    Target { item: matched.item(), matched, label }
                });
                ReviewFile {
                    row: number,
                    path: row.path.strip_prefix(&review.source).unwrap_or(&row.path).display().to_string(),
                    size: row.size,
                    target,
                    confidence: match row.confidence {
                        yokoku_domain::Confidence::Certain => Confidence::Certain,
                        yokoku_domain::Confidence::Guess => Confidence::Guess,
                        yokoku_domain::Confidence::Unknown => Confidence::Unknown,
                    },
                    skipped: row.skipped,
                    replace: row.replace,
                    conflicts: reviewed
                        .conflicts
                        .iter()
                        .map(|conflict| match conflict {
                            yokoku_media::Conflict::SharedTarget => Conflict::SharedTarget,
                            yokoku_media::Conflict::AlreadyHasFile => Conflict::AlreadyHasFile,
                        })
                        .collect(),
                }
            })
            .collect();
        Ok(Some(Review {
            id: review.id,
            source: review.source.display().to_string(),
            from_download: review.download.is_some(),
            rows,
        }))
    }

    pub(super) async fn match_file(
        reviewer: &Reviewer,
        import: ImportId,
        row: usize,
        target: Match,
    ) -> Result<(), ServerFnError> {
        let target = match target {
            Match::Episodes { series, season, first, last } => {
                let span = EpisodeSpan::new(season, first, last)
                    .ok_or_else(|| ServerFnError::new("The last episode comes before the first"))?;
                FileTarget::Episodes { series, span }
            },
            Match::Movie { id } => FileTarget::Movie(id),
        };
        reviewer.match_row(import, row, target).await.map_err(failure)
    }

    pub(super) async fn approve(reviewer: &Reviewer, import: ImportId) -> Result<Imported, ServerFnError> {
        match reviewer.approve(import).await.map_err(failure)? {
            Approval::Linked(files) => Ok(Imported::Linked(files.len())),
            Approval::Queued => Ok(Imported::Queued),
        }
    }

    /// The error's message for the user; unexpected ones go to the log.
    pub(super) fn failure(error: MediaError) -> ServerFnError {
        match error {
            MediaError::ImportNotFound(_) | MediaError::NotInReview(_) => {
                ServerFnError::new("It no longer waits for review; close it and reload the page")
            },
            MediaError::RowNotFound(_) => ServerFnError::new("That file is no longer in the import; reload it"),
            MediaError::ReplaceInPlace => ServerFnError::new("Only files from a download can replace library files"),
            MediaError::SeriesNotFound(_) | MediaError::MovieNotFound(_) => {
                ServerFnError::new("It is no longer in the library")
            },
            MediaError::EpisodesNotFound(span) => ServerFnError::new(format!("The series has no {span}")),
            MediaError::UnmatchedRows(rows) => ServerFnError::new(format!("Match or skip {} first", rows_named(&rows))),
            MediaError::ConflictingRows(rows) => {
                ServerFnError::new(format!("Resolve the conflicts of {} first", rows_named(&rows)))
            },
            error => {
                error!(%error, "reviewing the import failed");
                ServerFnError::new("Something went wrong; the server log has the cause")
            },
        }
    }

    fn rows_named(rows: &[usize]) -> String {
        let numbers: Vec<String> = rows.iter().map(ToString::to_string).collect();
        match numbers.as_slice() {
            [one] => format!("file {one}"),
            _ => format!("files {}", numbers.join(", ")),
        }
    }

    impl From<FileTarget> for Match {
        fn from(target: FileTarget) -> Self {
            match target {
                FileTarget::Episodes { series, span } => {
                    Self::Episodes { series, season: span.season(), first: span.first(), last: span.last() }
                },
                FileTarget::Movie(id) => Self::Movie { id },
            }
        }
    }
}
