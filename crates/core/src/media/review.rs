use std::{path::PathBuf, sync::Arc};

use jiff::Timestamp;
use yokoku_domain::{Clock, Confidence, DownloadId, FileTarget, ImportId, MediaFileId, SeriesId};

use crate::{
    events::{FilesImported, Publisher, QueueChanges},
    media::{
        Import, ImportRow, ImportStatus, MediaError, MediaFile, Resolution,
        detect::{Conflict, ImportPlan, ListedFile, MatchScope},
        ports::{Catalog, Changes, MediaRepo},
    },
};

/// Manual matching of files detection was unsure about (FR-4.11, FR-8.3). Rows are numbered
/// from 1.
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
    /// Empty for skipped rows.
    pub conflicts: Vec<Conflict>,
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
        let conflicts = self.conflicts(&import).await?;
        let rows =
            import.rows.into_iter().zip(conflicts).map(|(row, conflicts)| ReviewRow { row, conflicts }).collect();
        Ok(ImportReview {
            id: import.id,
            source: import.source,
            download: import.download,
            created_at: import.created_at,
            rows,
        })
    }

    /// The episodes or movie must be in the library.
    pub async fn match_row(&self, id: ImportId, row: usize, target: FileTarget) -> Result<(), MediaError> {
        self.check_exists(target).await?;
        self.update_row(id, row, |row| {
            row.target = Some(target);
            row.skipped = false;
            row.resolution = Resolution::Unresolved;
        })
        .await
    }

    pub async fn skip_row(&self, id: ImportId, row: usize) -> Result<(), MediaError> {
        self.update_row(id, row, |row| row.skipped = true).await
    }

    /// Marks a row to replace the library file that holds its target; only for downloads.
    pub async fn replace_row(&self, id: ImportId, row: usize) -> Result<(), MediaError> {
        if self.pending_import(id).await?.download.is_none() {
            return Err(MediaError::ReplaceInPlace);
        }
        self.update_row(id, row, |row| {
            row.resolution = Resolution::Replace;
            row.skipped = false;
        })
        .await
    }

    /// Detects the files of `rows` again: against `series`, placing its names without a season in
    /// `season` when given, or against the whole library. Each keeps no match when nothing fits,
    /// and none keeps its resolution.
    pub async fn redetect(
        &self,
        id: ImportId,
        rows: &[usize],
        series: Option<SeriesId>,
        season: Option<u16>,
    ) -> Result<(), MediaError> {
        let mut import = self.pending_import(id).await?;
        let indexes: Vec<usize> = rows
            .iter()
            .map(|&row| {
                row.checked_sub(1).filter(|&index| index < import.rows.len()).ok_or(MediaError::RowNotFound(row))
            })
            .collect::<Result<_, _>>()?;
        let base = import.source.parent().unwrap_or(&import.source).to_owned();
        let files: Vec<ListedFile> = indexes
            .iter()
            .map(|&index| {
                let row = &import.rows[index];
                ListedFile { path: row.path.strip_prefix(&base).unwrap_or(&row.path).to_owned(), size: row.size }
            })
            .collect();

        let plan = match series {
            Some(id) => {
                let series = self.catalog.series(id).await?.ok_or(MediaError::SeriesNotFound(id))?;
                let scope = match season {
                    Some(season) => MatchScope::SeriesSeason { series: &series, season },
                    None => MatchScope::Series(&series),
                };
                ImportPlan::new(&files, scope)
            },
            None => {
                let (series, movies) = (self.catalog.all_series().await?, self.catalog.all_movies().await?);
                ImportPlan::new(&files, MatchScope::Library { series: &series, movies: &movies })
            },
        };
        for (index, file) in indexes.into_iter().zip(&files) {
            let detected = plan.rows.iter().find(|row| row.video.path == file.path);
            let row = &mut import.rows[index];
            row.target = detected.and_then(|row| row.target);
            row.confidence = detected.map_or(Confidence::Unknown, |row| row.confidence);
            row.skipped = false;
            row.resolution = Resolution::Unresolved;
        }
        self.repo.save(&Changes { imports: vec![import], ..Changes::default() }).await?;
        self.changes.notify();
        Ok(())
    }

    /// Marks a row to be imported beside the library file and other rows that hold its target.
    pub async fn keep_both_row(&self, id: ImportId, row: usize) -> Result<(), MediaError> {
        self.update_row(id, row, |row| {
            row.resolution = Resolution::KeepBoth;
            row.skipped = false;
        })
        .await
    }

    /// Every row that is not skipped needs a match free of conflicts. Files found by a scan are
    /// linked where they are; files from a download wait for `Importer` to place them.
    pub async fn approve(&self, id: ImportId) -> Result<Approval, MediaError> {
        let mut import = self.pending_import(id).await?;
        let conflicts = self.conflicts(&import).await?;
        let numbered = || import.rows.iter().zip(&conflicts).zip(1..).filter(|((row, _), _)| !row.skipped);

        let unmatched: Vec<usize> = numbered().filter(|((row, _), _)| row.target.is_none()).map(|(_, n)| n).collect();
        if !unmatched.is_empty() {
            return Err(MediaError::UnmatchedRows(unmatched));
        }
        let conflicting: Vec<usize> =
            numbered().filter(|((_, conflicts), _)| !conflicts.is_empty()).map(|(_, n)| n).collect();
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
                    target: row.target?,
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

    async fn update_row(
        &self,
        id: ImportId,
        row: usize,
        change: impl FnOnce(&mut ImportRow),
    ) -> Result<(), MediaError> {
        let mut import = self.pending_import(id).await?;
        let index =
            row.checked_sub(1).filter(|&index| index < import.rows.len()).ok_or(MediaError::RowNotFound(row))?;
        change(&mut import.rows[index]);
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

    /// Per row: another row holds the same episode or movie, or a library file already does and the
    /// row does not replace it. A row kept beside both has neither, and causes none.
    async fn conflicts(&self, import: &Import) -> Result<Vec<Vec<Conflict>>, MediaError> {
        let linked: Vec<FileTarget> = self.repo.files().await?.into_iter().map(|file| file.target).collect();
        let active = |row: &ImportRow| row.target.filter(|_| !row.skipped && row.resolution != Resolution::KeepBoth);

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
