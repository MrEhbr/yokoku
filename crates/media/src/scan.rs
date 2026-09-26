use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use jiff::Timestamp;
use yokoku_detect::{DownloadFile, Target, plan};
use yokoku_domain::{Clock, Confidence, FileTarget, ImportId, MediaFileId};
use yokoku_events::{DeleteReason, Event, LinkedFile};

use crate::{
    Import, ImportRow, ImportStatus, MediaError, MediaFile, RootKind,
    ports::{Catalog, Changes, FileSystem, MediaRepo},
};

pub struct Scanner {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
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

impl Scanner {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, catalog, fs, clock }
    }

    /// Links new files in every root folder, sends the rest to review and forgets linked files
    /// that are gone (FR-8.2, FR-8.3, FR-8.7). Each root folder is committed on its own; one
    /// that cannot be read stops the scan before anything in it changes.
    pub async fn scan(&self) -> Result<ScanReport, MediaError> {
        let series = self.catalog.all_series().await?;
        let movies = self.catalog.all_movies().await?;
        let claimed = self.claimed_paths().await?;

        let mut report = ScanReport::default();
        for root in self.repo.root_folders().await? {
            let listed = self.fs.files(&root.path).await?;
            let known = self.repo.files().await?;
            let target = match root.kind {
                RootKind::Series => Target::Library { series: &series, movies: &[] },
                RootKind::Movies => Target::Library { series: &[], movies: &movies },
            };
            let now = self.clock.now().timestamp();
            let (changes, events) = scan_root(&root.path, listed, &known, &claimed, target, now);

            self.repo.save(&changes, &events).await?;
            report.found += changes.added_files.len();
            report.vanished += changes.removed_files.len();
            report.needs_review.extend(changes.imports.iter().map(|import| import.id));
        }
        Ok(report)
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

fn scan_root(
    root: &Path,
    listed: Vec<DownloadFile>,
    known: &[MediaFile],
    claimed: &HashSet<PathBuf>,
    target: Target<'_>,
    now: Timestamp,
) -> (Changes, Vec<Event>) {
    let listed_paths: HashSet<&Path> = listed.iter().map(|file| file.path.as_path()).collect();
    let (vanished, kept): (Vec<&MediaFile>, Vec<&MediaFile>) =
        known.iter().partition(|file| file.path.starts_with(root) && !listed_paths.contains(file.path.as_path()));
    let known_paths: HashSet<&Path> = known.iter().map(|file| file.path.as_path()).collect();
    let mut occupied: Vec<FileTarget> = kept.iter().map(|file| file.target).collect();

    let mut changes = Changes::default();
    let new_files =
        listed.into_iter().filter(|file| !known_paths.contains(file.path.as_path()) && !claimed.contains(&file.path));
    for (entry, files) in group_by_entry(root, new_files) {
        let mut rows = Vec::new();
        for row in plan(&files, target).rows {
            let path = root.join(&row.video.path);
            let size = row.video.size;
            let certain = row.confidence == Confidence::Certain && row.conflicts.is_empty();
            match row.target {
                Some(target) if certain && !occupied.iter().any(|taken| taken.overlaps(&target)) => {
                    occupied.push(target);
                    changes.added_files.push(MediaFile {
                        id: MediaFileId::generate(),
                        path,
                        size,
                        target,
                        added_at: now,
                    });
                },
                target => rows.push(ImportRow {
                    path,
                    size,
                    target,
                    confidence: row.confidence,
                    skipped: false,
                    replace: false,
                }),
            }
        }
        if !rows.is_empty() {
            let id = ImportId::generate();
            changes.imports.push(Import {
                id,
                source: root.join(entry),
                download: None,
                status: ImportStatus::NeedsReview,
                error: None,
                rows,
                created_at: now,
            });
        }
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

/// Files grouped by the root folder entry they are in, with paths relative to the root.
fn group_by_entry(root: &Path, files: impl Iterator<Item = DownloadFile>) -> BTreeMap<PathBuf, Vec<DownloadFile>> {
    let mut groups: BTreeMap<PathBuf, Vec<DownloadFile>> = BTreeMap::new();
    for file in files {
        let Ok(relative) = file.path.strip_prefix(root) else { continue };
        let Some(entry) = relative.components().next() else { continue };
        let relative = relative.to_owned();
        groups
            .entry(PathBuf::from(entry.as_os_str()))
            .or_default()
            .push(DownloadFile { path: relative, size: file.size });
    }
    groups
}

impl MediaFile {
    pub(crate) fn linked(&self) -> LinkedFile {
        LinkedFile { file: self.id, path: self.path.clone(), target: self.target }
    }
}
