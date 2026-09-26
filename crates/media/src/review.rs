use std::{path::PathBuf, sync::Arc};

use jiff::Timestamp;
use yokoku_detect::Conflict;
use yokoku_domain::{Clock, FileTarget, ImportId, MediaFileId};
use yokoku_events::Event;

use crate::{
    Import, ImportRow, ImportStatus, MediaError, MediaFile,
    ports::{Catalog, Changes, MediaRepo},
};

/// Manual matching of files detection was unsure about (FR-4.11, FR-8.3). Rows are numbered
/// from 1.
pub struct Review {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReview {
    pub id: ImportId,
    pub source: PathBuf,
    pub created_at: Timestamp,
    pub rows: Vec<ReviewRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRow {
    pub row: ImportRow,
    /// Empty for skipped rows.
    pub conflicts: Vec<Conflict>,
}

impl Review {
    pub fn new(repo: Arc<dyn MediaRepo>, catalog: Arc<dyn Catalog>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, catalog, clock }
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
        Ok(ImportReview { id: import.id, source: import.source, created_at: import.created_at, rows })
    }

    /// The episodes or movie must be in the library.
    pub async fn match_row(&self, id: ImportId, row: usize, target: FileTarget) -> Result<(), MediaError> {
        self.check_exists(target).await?;
        self.update_row(id, row, |row| {
            row.target = Some(target);
            row.skipped = false;
        })
        .await
    }

    pub async fn skip_row(&self, id: ImportId, row: usize) -> Result<(), MediaError> {
        self.update_row(id, row, |row| row.skipped = true).await
    }

    /// Links every row that is not skipped; all of them need a match free of conflicts.
    pub async fn approve(&self, id: ImportId) -> Result<Vec<MediaFile>, MediaError> {
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
        let event = Event::FilesImported { import: id, files: files.iter().map(MediaFile::linked).collect() };
        let changes = Changes { added_files: files.clone(), imports: vec![import], ..Changes::default() };
        self.repo.save(&changes, &[event]).await?;
        Ok(files)
    }

    async fn pending_import(&self, id: ImportId) -> Result<Import, MediaError> {
        let import = self.repo.import(id).await?.ok_or(MediaError::ImportNotFound(id))?;
        match import.status {
            ImportStatus::NeedsReview => Ok(import),
            ImportStatus::Done => Err(MediaError::ImportDone(id)),
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
        let changes = Changes { imports: vec![import], ..Changes::default() };
        Ok(self.repo.save(&changes, &[]).await?)
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

    /// Per row: another row holds the same episode or movie, or a library file already does.
    async fn conflicts(&self, import: &Import) -> Result<Vec<Vec<Conflict>>, MediaError> {
        let linked: Vec<FileTarget> = self.repo.files().await?.into_iter().map(|file| file.target).collect();
        let active = |row: &ImportRow| row.target.filter(|_| !row.skipped);

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
                let taken = linked.iter().any(|file| file.overlaps(&target));
                [(shared, Conflict::SharedTarget), (taken, Conflict::AlreadyHasFile)]
                    .into_iter()
                    .filter_map(|(present, conflict)| present.then_some(conflict))
                    .collect()
            })
            .collect())
    }
}
