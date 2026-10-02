use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    path::PathBuf,
    sync::Arc,
};

use jiff::Timestamp;
use yokoku_domain::{
    Clock, Confidence, DownloadId, EpisodeRef, EpisodeSpan, FileTarget, ImportId, ItemId, MediaFileId, Series,
    SeriesId, events::FilesImported,
};

use crate::{
    events::{Publisher, QueueChanges},
    media::{
        Episodes, Import, ImportRow, ImportStatus, MediaError, MediaFile, Resolution, RowMatch,
        detect::{Conflict, ImportPlan, ListedFile, MatchScope},
        ports::{Catalog, Changes, MediaRepo},
    },
};

/// Manual matching of files detection was unsure about. Rows are numbered
/// from 1; a skipped row is unchecked and left where it is.
pub struct Reviewer {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    clock: Arc<dyn Clock>,
    events: Publisher,
    changes: QueueChanges,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReview {
    pub id: ImportId,
    pub source: PathBuf,
    pub download: Option<DownloadId>,
    pub created_at: Timestamp,
    pub rows: Vec<ReviewRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approval {
    /// Found by a scan: linked where they are.
    Linked(Vec<MediaFile>),
    /// From a download: waiting to be placed.
    Queued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRow {
    pub row: ImportRow,
    /// Why the row has no complete match in the library.
    pub problem: Option<Problem>,
    /// Empty for skipped rows.
    pub conflicts: Vec<Conflict>,
}

/// What a row's match lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    NoMatch,
    NoSeason,
    NoEpisodes,
    /// The series or movie is no longer in the library.
    Gone,
    SeasonNotInSeries(u16),
    EpisodesNotInSeries(EpisodeSpan),
}

impl Reviewer {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        clock: Arc<dyn Clock>,
        events: Publisher,
        changes: QueueChanges,
    ) -> Self {
        Self { repo, catalog, clock, events, changes }
    }

    /// Imports waiting for review, oldest first.
    pub async fn pending(&self) -> Result<Vec<Import>, MediaError> {
        Ok(self.repo.imports(ImportStatus::NeedsReview).await?)
    }

    pub async fn get(&self, id: ImportId) -> Result<ImportReview, MediaError> {
        let import = self.pending_import(id).await?;
        let problems = self.problems(&import).await?;
        let conflicts = self.conflicts(&import).await?;
        let rows = import
            .rows
            .into_iter()
            .zip(problems)
            .zip(conflicts)
            .map(|((row, problem), conflicts)| ReviewRow { row, problem, conflicts })
            .collect();
        Ok(ImportReview {
            id: import.id,
            source: import.source,
            download: import.download,
            created_at: import.created_at,
            rows,
        })
    }

    /// The episodes or movie must be in the library; the row is checked.
    pub async fn match_row(&self, id: ImportId, row: usize, target: FileTarget) -> Result<(), MediaError> {
        self.check_exists(target).await?;
        self.update_rows(id, &[row], |rows| {
            for (_, row) in rows {
                row.matched = target.into();
                row.confirm();
            }
            Ok(())
        })
        .await
    }

    /// Checks or unchecks the rows.
    pub async fn include_rows(&self, id: ImportId, rows: &[usize], included: bool) -> Result<(), MediaError> {
        self.update_rows(id, rows, |rows| {
            for (_, row) in rows {
                row.skipped = !included;
            }
            Ok(())
        })
        .await
    }

    /// Detects the rows again against `series`; each keeps the series and its names' numbers
    /// when its episodes are not found. The rows are checked.
    pub async fn set_series(&self, id: ImportId, rows: &[usize], series: SeriesId) -> Result<(), MediaError> {
        let series = self.catalog.series(series).await?.ok_or(MediaError::SeriesNotFound(series))?;
        let source = self.pending_import(id).await?.source;
        let base = source.parent().unwrap_or(&source).to_owned();
        self.update_rows(id, rows, |rows| {
            let files: Vec<ListedFile> = rows
                .iter()
                .map(|(_, row)| ListedFile {
                    path: row.path.strip_prefix(&base).unwrap_or(&row.path).to_owned(),
                    size: row.size,
                })
                .collect();
            let plan = ImportPlan::new(&files, MatchScope::Series(&series));
            for ((_, row), file) in rows.iter_mut().zip(&files) {
                let detected = plan.rows.iter().find(|detected| detected.video.path == file.path);
                row.matched = detected
                    .map_or(RowMatch::Series { series: series.id, season: None, episodes: None }, |detected| {
                        detected.row_match(None)
                    });
                row.confidence = detected.map_or(Confidence::Unknown, |detected| detected.confidence);
                row.skipped = false;
                row.resolution = Resolution::Unresolved;
            }
            Ok(())
        })
        .await
    }

    /// Places the rows' episodes in `season`, keeping their numbers; fails without a change when a
    /// row has no series.
    pub async fn set_season(&self, id: ImportId, rows: &[usize], season: u16) -> Result<(), MediaError> {
        self.update_rows(id, rows, |rows| {
            let without: Vec<usize> = rows
                .iter()
                .filter(|(_, row)| !matches!(row.matched, RowMatch::Series { .. }))
                .map(|(number, _)| *number)
                .collect();
            if !without.is_empty() {
                return Err(MediaError::RowsWithoutSeries(without));
            }
            for (_, row) in rows {
                if let RowMatch::Series { season: current, .. } = &mut row.matched {
                    *current = Some(season);
                }
                row.confirm();
            }
            Ok(())
        })
        .await
    }

    /// Gives the rows `episodes` of `series` in order, one each, for files numbered differently
    /// from the series, like a season released in parts that each start at 1.
    pub async fn set_episodes(
        &self,
        id: ImportId,
        rows: &[usize],
        series: SeriesId,
        episodes: &[EpisodeRef],
    ) -> Result<(), MediaError> {
        if episodes.len() != rows.len() {
            return Err(MediaError::EpisodeCount { rows: rows.len(), episodes: episodes.len() });
        }
        let found = self.catalog.series(series).await?.ok_or(MediaError::SeriesNotFound(series))?;
        if let Some(&missing) = episodes.iter().find(|&&episode| found.episode(episode).is_none()) {
            return Err(MediaError::EpisodesNotFound(EpisodeSpan::single(missing)));
        }
        self.update_rows(id, rows, |rows| {
            for ((_, row), &episode) in rows.iter_mut().zip(episodes) {
                row.matched = FileTarget::Episodes { series, span: EpisodeSpan::single(episode) }.into();
                row.confirm();
            }
            Ok(())
        })
        .await
    }

    /// Marks a row to replace the library file that holds its target; only for downloads.
    pub async fn replace_row(&self, id: ImportId, row: usize) -> Result<(), MediaError> {
        if self.pending_import(id).await?.download.is_none() {
            return Err(MediaError::ReplaceInPlace);
        }
        self.update_rows(id, &[row], |rows| {
            for (_, row) in rows {
                row.resolution = Resolution::Replace;
                row.skipped = false;
            }
            Ok(())
        })
        .await
    }

    /// Marks a row to be imported beside the library file and other rows that hold its target.
    pub async fn keep_both_row(&self, id: ImportId, row: usize) -> Result<(), MediaError> {
        self.update_rows(id, &[row], |rows| {
            for (_, row) in rows {
                row.resolution = Resolution::KeepBoth;
                row.skipped = false;
            }
            Ok(())
        })
        .await
    }

    /// Every checked row needs a match in the library, free of conflicts. Files found by a scan
    /// are linked where they are; files from a download wait for `Importer` to place them.
    pub async fn approve(&self, id: ImportId) -> Result<Approval, MediaError> {
        let mut import = self.pending_import(id).await?;
        let problems = self.problems(&import).await?;
        let conflicts = self.conflicts(&import).await?;
        let checked = |flags: Vec<bool>| -> Vec<usize> {
            import
                .rows
                .iter()
                .zip(flags)
                .zip(1..)
                .filter(|((row, flag), _)| !row.skipped && *flag)
                .map(|(_, n)| n)
                .collect()
        };

        let unmatched = checked(problems.iter().map(Option::is_some).collect());
        if !unmatched.is_empty() {
            return Err(MediaError::UnmatchedRows(unmatched));
        }
        let conflicting = checked(conflicts.iter().map(|conflicts| !conflicts.is_empty()).collect());
        if !conflicting.is_empty() {
            return Err(MediaError::ConflictingRows(conflicting));
        }

        if import.download.is_some() {
            import.status = ImportStatus::Approved;
            self.repo.save(&Changes { imports: vec![import], ..Changes::default() }).await?;
            self.changes.notify();
            return Ok(Approval::Queued);
        }

        let now = self.clock.now().timestamp();
        let files: Vec<MediaFile> = import
            .rows
            .iter()
            .filter(|row| !row.skipped)
            .filter_map(|row| {
                Some(MediaFile {
                    id: MediaFileId::generate(),
                    path: row.path.clone(),
                    size: row.size,
                    target: row.target()?,
                    added_at: now,
                })
            })
            .collect();
        import.status = ImportStatus::Done;
        let event = FilesImported {
            import: id,
            download: import.download,
            files: files.iter().map(MediaFile::linked).collect(),
        };
        let changes = Changes { added_files: files.clone(), imports: vec![import], ..Changes::default() };
        self.repo.save(&changes).await?;
        self.changes.notify();
        self.events.publish(event).await;
        Ok(Approval::Linked(files))
    }

    async fn pending_import(&self, id: ImportId) -> Result<Import, MediaError> {
        let import = self.repo.import(id).await?.ok_or(MediaError::ImportNotFound(id))?;
        match import.status {
            ImportStatus::NeedsReview => Ok(import),
            _ => Err(MediaError::NotInReview(id)),
        }
    }

    /// Applies `change` to the rows numbered `rows`, in that order, and saves the import.
    async fn update_rows(
        &self,
        id: ImportId,
        rows: &[usize],
        change: impl FnOnce(&mut [(usize, &mut ImportRow)]) -> Result<(), MediaError>,
    ) -> Result<(), MediaError> {
        let mut import = self.pending_import(id).await?;
        let count = import.rows.len();
        for &row in rows {
            row.checked_sub(1).filter(|&index| index < count).ok_or(MediaError::RowNotFound(row))?;
        }
        let mut by_index: Vec<Option<&mut ImportRow>> = import.rows.iter_mut().map(Some).collect();
        let mut chosen: Vec<(usize, &mut ImportRow)> =
            rows.iter().filter_map(|&row| Some((row, by_index[row - 1].take()?))).collect();
        change(&mut chosen)?;
        self.repo.save(&Changes { imports: vec![import], ..Changes::default() }).await?;
        self.changes.notify();
        Ok(())
    }

    async fn check_exists(&self, target: FileTarget) -> Result<(), MediaError> {
        match target {
            FileTarget::Episodes { series: id, span } => {
                let series = self.catalog.series(id).await?.ok_or(MediaError::SeriesNotFound(id))?;
                if span.refs().any(|reference| series.episode(reference).is_none()) {
                    return Err(MediaError::EpisodesNotFound(span));
                }
            },
            FileTarget::Movie(id) => {
                self.catalog.movie(id).await?.ok_or(MediaError::MovieNotFound(id))?;
            },
        }
        Ok(())
    }

    /// Per row: what its match lacks in the library.
    async fn problems(&self, import: &Import) -> Result<Vec<Option<Problem>>, MediaError> {
        let mut series: HashMap<SeriesId, Option<Series>> = HashMap::new();
        for row in &import.rows {
            if let RowMatch::Series { series: id, .. } = row.matched
                && let Entry::Vacant(slot) = series.entry(id)
            {
                slot.insert(self.catalog.series(id).await?);
            }
        }
        let mut problems = Vec::with_capacity(import.rows.len());
        for row in &import.rows {
            problems.push(match row.matched {
                RowMatch::None => Some(Problem::NoMatch),
                RowMatch::Movie(id) => self.catalog.movie(id).await?.is_none().then_some(Problem::Gone),
                RowMatch::Series { series: id, season, episodes } => match (series[&id].as_ref(), season, episodes) {
                    (None, _, _) => Some(Problem::Gone),
                    (Some(_), None, _) => Some(Problem::NoSeason),
                    (Some(found), Some(season), _) if !found.seasons.iter().any(|known| known.number == season) => {
                        Some(Problem::SeasonNotInSeries(season))
                    },
                    (Some(_), Some(_), None) => Some(Problem::NoEpisodes),
                    (Some(found), Some(season), Some(Episodes { first, last })) => {
                        match EpisodeSpan::new(season, first, last) {
                            Some(span) => span
                                .refs()
                                .any(|reference| found.episode(reference).is_none())
                                .then_some(Problem::EpisodesNotInSeries(span)),
                            None => Some(Problem::NoEpisodes),
                        }
                    },
                },
            });
        }
        Ok(problems)
    }

    /// Per row: another row holds the same episode or movie, or a library file already does and the
    /// row does not replace it. A row kept beside both has neither, and causes none.
    async fn conflicts(&self, import: &Import) -> Result<Vec<Vec<Conflict>>, MediaError> {
        let active = |row: &ImportRow| row.target().filter(|_| !row.skipped && row.resolution != Resolution::KeepBoth);
        let items: HashSet<ItemId> = import.rows.iter().filter_map(active).map(|target| target.item()).collect();
        let mut linked: Vec<FileTarget> = Vec::new();
        for item in items {
            linked.extend(self.repo.files_of(item).await?.into_iter().map(|file| file.target));
        }

        Ok(import
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let Some(target) = active(row) else { return Vec::new() };
                let shared = import
                    .rows
                    .iter()
                    .enumerate()
                    .any(|(other, row)| other != index && active(row).is_some_and(|other| other.overlaps(&target)));
                let taken =
                    row.resolution == Resolution::Unresolved && linked.iter().any(|file| file.overlaps(&target));
                [(shared, Conflict::SharedTarget), (taken, Conflict::AlreadyHasFile)]
                    .into_iter()
                    .filter_map(|(present, conflict)| present.then_some(conflict))
                    .collect()
            })
            .collect())
    }
}
