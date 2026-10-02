//! Matching the files detection was unsure about, then importing them.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ImportId, ItemId, MovieId, SeriesId};

#[cfg(feature = "server")]
use crate::api::{Dep, Importer, Library, Reviewer};

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
    /// Checked: imported; unchecked files are left where they are.
    pub included: bool,
    /// The series or movie it is matched to, while in the library.
    pub item: Option<Item>,
    pub season: Option<u16>,
    /// `3 · Fifty Shades of Green`, or `3–4 · …` for a multi-episode file.
    pub episodes: Option<String>,
    /// Its complete match, to start the editor from.
    pub matched: Option<Match>,
    /// Detection guessed the match from an unclear name.
    pub guessed: bool,
    /// What the match lacks, like "Futurama has no season 13".
    pub problem: Option<String>,
    pub resolution: Resolution,
    /// Why it cannot be imported as matched, like "already has a file".
    pub conflicts: Vec<Conflict>,
    /// Its path once imported, relative to its item's folder; none while unchecked or not fully
    /// matched, and for files a scan found, which stay where they are.
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub title: String,
}

/// How a file settles a library file, or another file, that holds its match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Resolution {
    /// Either is a conflict.
    Unresolved,
    /// Replaces the library file.
    Replace,
    /// Imported beside both; a download's file takes a numbered name where its own is taken.
    KeepBoth,
}

/// What a file is matched to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Match {
    Episodes { series: SeriesId, season: u16, first: u16, last: u16 },
    Movie { id: MovieId },
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
#[post("/api/review", reviewer: Dep<Reviewer>, importer: Dep<Importer>, library: Dep<Library>)]
pub async fn review(import: ImportId) -> Result<Option<Review>, ServerFnError> {
    server::review(&reviewer, &importer, &library, import).await
}

/// The file is checked.
#[post("/api/review/match", reviewer: Dep<Reviewer>)]
pub async fn match_file(import: ImportId, row: usize, target: Match) -> Result<(), ServerFnError> {
    server::match_file(&reviewer, import, row, target).await
}

/// Checks or unchecks the files.
#[post("/api/review/include", reviewer: Dep<Reviewer>)]
pub async fn include_files(import: ImportId, rows: Vec<usize>, included: bool) -> Result<(), ServerFnError> {
    reviewer.include_rows(import, &rows, included).await.map_err(server::failure)
}

/// Detects the files again against `series`, keeping it where their episodes are not found.
#[post("/api/review/series", reviewer: Dep<Reviewer>)]
pub async fn set_series(import: ImportId, rows: Vec<usize>, series: SeriesId) -> Result<(), ServerFnError> {
    reviewer.set_series(import, &rows, series).await.map_err(server::failure)
}

/// Places the files' episodes in `season`, keeping their numbers.
#[post("/api/review/season", reviewer: Dep<Reviewer>)]
pub async fn set_season(import: ImportId, rows: Vec<usize>, season: u16) -> Result<(), ServerFnError> {
    reviewer.set_season(import, &rows, season).await.map_err(server::failure)
}

/// Gives the files `episodes` of `series`, as season and episode numbers, in order, one each.
#[post("/api/review/episodes", reviewer: Dep<Reviewer>)]
pub async fn set_episodes(
    import: ImportId,
    rows: Vec<usize>,
    series: SeriesId,
    episodes: Vec<(u16, u16)>,
) -> Result<(), ServerFnError> {
    let episodes: Vec<_> =
        episodes.into_iter().map(|(season, episode)| yokoku_domain::EpisodeRef { season, episode }).collect();
    reviewer.set_episodes(import, &rows, series, &episodes).await.map_err(server::failure)
}

/// The downloaded file replaces the library file of its episode or movie.
#[post("/api/review/replace", reviewer: Dep<Reviewer>)]
pub async fn replace_file(import: ImportId, row: usize) -> Result<(), ServerFnError> {
    reviewer.replace_row(import, row).await.map_err(server::failure)
}

/// The file is imported beside the library file and other files that hold its match.
#[post("/api/review/keep-both", reviewer: Dep<Reviewer>)]
pub async fn keep_both_file(import: ImportId, row: usize) -> Result<(), ServerFnError> {
    reviewer.keep_both_row(import, row).await.map_err(server::failure)
}

/// Every checked file needs a match free of conflicts.
#[post("/api/review/approve", reviewer: Dep<Reviewer>)]
pub async fn approve(import: ImportId) -> Result<Imported, ServerFnError> {
    server::approve(&reviewer, import).await
}

#[cfg(feature = "server")]
mod server {
    use std::collections::{HashMap, hash_map::Entry};

    use dioxus::{
        logger::tracing::{error, warn},
        prelude::*,
    };
    use yokoku_core::{
        library::{LibraryFilter, LibrarySort},
        media::{Approval, Episodes, ImportRow, MediaError, Problem, RowMatch},
    };
    use yokoku_domain::{Confidence, EpisodeRef, EpisodeSpan, FileTarget, ImportId, ItemId, Series, SeriesId};

    use super::{Conflict, Imported, Importer, Item, Library, Match, Resolution, Review, ReviewFile, Reviewer};
    use crate::api::unexpected;

    pub(super) async fn review(
        reviewer: &Reviewer,
        importer: &Importer,
        library: &Library,
        import: ImportId,
    ) -> Result<Option<Review>, ServerFnError> {
        let review = match reviewer.get(import).await {
            Ok(review) => review,
            Err(MediaError::ImportNotFound(_) | MediaError::NotInReview(_)) => return Ok(None),
            Err(error) => return Err(failure(error)),
        };
        let nameable: Vec<ImportRow> = review
            .rows
            .iter()
            .map(|reviewed| ImportRow {
                skipped: reviewed.row.skipped || reviewed.problem.is_some(),
                ..reviewed.row.clone()
            })
            .collect();
        let names = match review.download {
            Some(_) => importer.destinations(&nameable).await.unwrap_or_else(|error| {
                warn!(%error, "naming the files to import failed");
                Vec::new()
            }),
            None => Vec::new(),
        };
        let items = library.list(LibraryFilter::default(), LibrarySort::Title).await.map_err(|error| {
            error!(%error, "listing the library failed");
            ServerFnError::new("The library could not be loaded")
        })?;
        let titles: HashMap<ItemId, String> = items.into_iter().map(|entry| (entry.id, entry.title)).collect();
        let mut series: HashMap<SeriesId, Option<Series>> = HashMap::new();
        for reviewed in &review.rows {
            if let RowMatch::Series { series: id, .. } = reviewed.row.matched
                && let Entry::Vacant(slot) = series.entry(id)
            {
                slot.insert(library.series(id).await.ok());
            }
        }

        let rows = (1..)
            .zip(review.rows)
            .map(|(number, reviewed)| {
                let row = reviewed.row;
                let item_id = match row.matched {
                    RowMatch::None => None,
                    RowMatch::Series { series, .. } => Some(ItemId::Series(series)),
                    RowMatch::Movie(movie) => Some(ItemId::Movie(movie)),
                };
                let title = item_id.and_then(|id| titles.get(&id).cloned());
                let (season, episodes) = match row.matched {
                    RowMatch::Series { series: id, season, episodes } => {
                        (season, episodes.map(|episodes| episode_label(series[&id].as_ref(), season, episodes)))
                    },
                    _ => (None, None),
                };
                let name =
                    names.get(number - 1).cloned().flatten().map(|destination| destination.name.display().to_string());
                ReviewFile {
                    row: number,
                    path: row.path.strip_prefix(&review.source).unwrap_or(&row.path).display().to_string(),
                    size: row.size,
                    included: !row.skipped,
                    item: item_id.zip(title.clone()).map(|(id, title)| Item { id, title }),
                    season,
                    episodes,
                    matched: row.target().filter(|_| reviewed.problem.is_none()).map(Match::from),
                    guessed: row.confidence == Confidence::Guess,
                    problem: reviewed.problem.map(|problem| problem_label(problem, title.as_deref().unwrap_or("It"))),
                    resolution: match row.resolution {
                        yokoku_core::media::Resolution::Unresolved => Resolution::Unresolved,
                        yokoku_core::media::Resolution::Replace => Resolution::Replace,
                        yokoku_core::media::Resolution::KeepBoth => Resolution::KeepBoth,
                    },
                    conflicts: reviewed
                        .conflicts
                        .iter()
                        .map(|conflict| match conflict {
                            yokoku_core::media::Conflict::SharedTarget => Conflict::SharedTarget,
                            yokoku_core::media::Conflict::AlreadyHasFile => Conflict::AlreadyHasFile,
                        })
                        .collect(),
                    name,
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

    /// `3 · Title` or `3–4 · Title`, the title of the first episode when the series has it.
    fn episode_label(series: Option<&Series>, season: Option<u16>, Episodes { first, last }: Episodes) -> String {
        let numbers = if first == last { first.to_string() } else { format!("{first}–{last}") };
        let title = season
            .zip(series)
            .and_then(|(season, series)| series.episode(EpisodeRef { season, episode: first }))
            .map(|episode| episode.title.clone())
            .filter(|title| !title.is_empty());
        match title {
            Some(title) => format!("{numbers} · {title}"),
            None => numbers,
        }
    }

    fn problem_label(problem: Problem, title: &str) -> String {
        match problem {
            Problem::NoMatch => "Not matched".to_owned(),
            Problem::NoSeason => "Set the season".to_owned(),
            Problem::NoEpisodes => "Set the episodes".to_owned(),
            Problem::Gone => "No longer in the library".to_owned(),
            Problem::SeasonNotInSeries(season) => format!("{title} has no season {season}"),
            Problem::EpisodesNotInSeries(span) => format!("{title} has no {span}"),
        }
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
            MediaError::UnmatchedRows(rows) => {
                ServerFnError::new(format!("Match or uncheck {} first", rows_named(&rows)))
            },
            MediaError::ConflictingRows(rows) => {
                ServerFnError::new(format!("Resolve the conflicts of {} first", rows_named(&rows)))
            },
            MediaError::RowsWithoutSeries(rows) => {
                ServerFnError::new(format!("Set the series of {} first", rows_named(&rows)))
            },
            MediaError::EpisodeCount { rows, episodes } => ServerFnError::new(format!(
                "Choose {rows} {} for the checked files, not {episodes}",
                if rows == 1 { "episode" } else { "episodes" }
            )),
            error => unexpected(&error, "reviewing the import"),
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
