use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_detect::{DownloadFile, ImportPlan, Target};
use yokoku_domain::{Clock, Confidence, FileTarget, ImportId, ItemFolder, ItemId, MediaFileId};
use yokoku_events::{DeleteReason, Event, HandlerError, Recorded, Subscriber};

use crate::{
    Import, ImportRow, ImportStatus, MediaError, MediaFile,
    ports::{Catalog, Changes, FileSystem, LibraryLock, MediaRepo},
};

pub struct Scanner {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScanReport {
    /// New files linked to the library.
    pub found: usize,
    /// Linked files no longer on disk.
    pub vanished: usize,
    /// Imports created for files the user has to match.
    pub needs_review: Vec<ImportId>,
}

/// Library files and the files already taken by imports, read once per scan.
struct Known {
    files: Vec<MediaFile>,
    claimed: HashSet<PathBuf>,
}

impl Scanner {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, catalog, fs, lock, clock }
    }

    /// Links new files in the folder of every library item, sends the rest to review and forgets
    /// linked files that are gone (FR-8.2, FR-8.3, FR-8.7). Nothing else in a root folder is read.
    /// Each item folder is committed on its own; a root folder that cannot be read stops the scan
    /// before anything in it changes.
    pub async fn scan(&self) -> Result<ScanReport, MediaError> {
        let _lock = self.lock.acquire().await?;
        let series = self.catalog.all_series().await?;
        let movies = self.catalog.all_movies().await?;
        let known = self.known().await?;

        let items = series
            .iter()
            .map(|series| (&series.folder, Target::Series(series)))
            .chain(movies.iter().map(|movie| (&movie.folder, Target::Movie(movie))));
        let mut report = ScanReport::default();
        for (folder, target) in items {
            self.scan_folder(folder, target, &known, &mut report).await?;
        }
        Ok(report)
    }

    /// Scans the folder of one item, as `scan` does (FR-8.8); an item no longer in the library is
    /// left alone.
    pub async fn scan_item(&self, item: ItemId) -> Result<ScanReport, MediaError> {
        let _lock = self.lock.acquire().await?;
        let known = self.known().await?;
        let mut report = ScanReport::default();
        match item {
            ItemId::Series(id) => {
                if let Some(series) = self.catalog.series(id).await? {
                    self.scan_folder(&series.folder, Target::Series(&series), &known, &mut report).await?;
                }
            },
            ItemId::Movie(id) => {
                if let Some(movie) = self.catalog.movie(id).await? {
                    self.scan_folder(&movie.folder, Target::Movie(&movie), &known, &mut report).await?;
                }
            },
        }
        Ok(report)
    }

    /// A missing item folder holds no files; a missing root folder fails.
    async fn scan_folder(
        &self,
        folder: &ItemFolder,
        target: Target<'_>,
        known: &Known,
        report: &mut ScanReport,
    ) -> Result<(), MediaError> {
        if !self.fs.is_dir(&folder.root).await? {
            return Err(MediaError::NotAFolder(folder.root.clone()));
        }
        let path = folder.path();
        let listed = if self.fs.is_dir(&path).await? { self.fs.files(&path).await? } else { Vec::new() };
        let now = self.clock.now().timestamp();
        let (changes, events) = scan_folder(&folder.root, &path, listed, known, target, now);

        self.repo.save(&changes, &events).await?;
        report.found += changes.added_files.len();
        report.vanished += changes.removed_files.len();
        report.needs_review.extend(changes.imports.iter().map(|import| import.id));
        Ok(())
    }

    async fn known(&self) -> Result<Known, MediaError> {
        Ok(Known { files: self.repo.files().await?, claimed: self.claimed_paths().await? })
    }

    /// Files of imports not yet done, and files the user chose to skip.
    async fn claimed_paths(&self) -> Result<HashSet<PathBuf>, MediaError> {
        let mut pending = Vec::new();
        for status in [ImportStatus::NeedsReview, ImportStatus::Approved, ImportStatus::Importing, ImportStatus::Failed]
        {
            pending.extend(self.repo.imports(status).await?);
        }
        let done = self.repo.imports(ImportStatus::Done).await?;
        let pending_rows = pending.into_iter().flat_map(|import| import.rows);
        let skipped_rows = done.into_iter().flat_map(|import| import.rows).filter(|row| row.skipped);
        Ok(pending_rows.chain(skipped_rows).map(|row| row.path).collect())
    }
}

#[async_trait]
impl Subscriber for Scanner {
    fn name(&self) -> &'static str {
        "media.scan_added"
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        let item = match &recorded.event {
            Event::SeriesAdded { series, .. } => ItemId::Series(*series),
            Event::MovieAdded { movie, .. } => ItemId::Movie(*movie),
            _ => return Ok(()),
        };
        self.scan_item(item).await?;
        Ok(())
    }
}

/// Detection sees paths relative to `root`.
fn scan_folder(
    root: &Path,
    folder: &Path,
    listed: Vec<DownloadFile>,
    known: &Known,
    target: Target<'_>,
    now: Timestamp,
) -> (Changes, Vec<Event>) {
    let listed_paths: HashSet<&Path> = listed.iter().map(|file| file.path.as_path()).collect();
    let (vanished, kept): (Vec<&MediaFile>, Vec<&MediaFile>) = known
        .files
        .iter()
        .partition(|file| file.path.starts_with(folder) && !listed_paths.contains(file.path.as_path()));
    let known_paths: HashSet<&Path> = known.files.iter().map(|file| file.path.as_path()).collect();
    let mut occupied: Vec<FileTarget> = kept.iter().map(|file| file.target).collect();

    let new_files: Vec<DownloadFile> = listed
        .into_iter()
        .filter(|file| !known_paths.contains(file.path.as_path()) && !known.claimed.contains(&file.path))
        .filter_map(|file| Some(DownloadFile { path: file.path.strip_prefix(root).ok()?.to_owned(), size: file.size }))
        .collect();

    let mut changes = Changes::default();
    let mut rows = Vec::new();
    for row in ImportPlan::new(&new_files, target).rows {
        let path = root.join(&row.video.path);
        let size = row.video.size;
        let certain = row.confidence == Confidence::Certain && row.conflicts.is_empty();
        match row.target {
            Some(target) if certain && !occupied.iter().any(|taken| taken.overlaps(&target)) => {
                occupied.push(target);
                changes.added_files.push(MediaFile { id: MediaFileId::generate(), path, size, target, added_at: now });
            },
            target => {
                rows.push(ImportRow { path, size, target, confidence: row.confidence, skipped: false, replace: false })
            },
        }
    }
    if !rows.is_empty() {
        changes.imports.push(Import {
            id: ImportId::generate(),
            source: folder.to_owned(),
            download: None,
            status: ImportStatus::NeedsReview,
            error: None,
            rows,
            created_at: now,
        });
    }

    let mut events: Vec<Event> = vanished
        .iter()
        .map(|file| Event::FileDeleted {
            file: file.id,
            path: file.path.clone(),
            target: file.target,
            reason: DeleteReason::External,
        })
        .collect();
    if !changes.added_files.is_empty() {
        events.push(Event::FilesFound { files: changes.added_files.iter().map(MediaFile::linked).collect() });
    }
    events.extend(
        changes
            .imports
            .iter()
            .map(|import| Event::ImportNeedsReview { import: import.id, source: import.source.clone() }),
    );
    changes.removed_files = vanished.iter().map(|file| file.id).collect();
    (changes, events)
}
