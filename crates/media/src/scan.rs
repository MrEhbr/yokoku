use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use jiff::Timestamp;
use tracing::{info, instrument};
use yokoku_detect::{ImportPlan, ListedFile, MatchScope};
use yokoku_domain::{Clock, Confidence, FileTarget, ImportId, ItemFolder, ItemId, MediaFileId};
use yokoku_events::{
    DeleteReason, EpisodesRenumbered, Event, FileDeleted, FilesFound, Handler, HandlerError, ImportNeedsReview,
    MovieAdded, Publisher, SeriesAdded,
};

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
    events: Publisher,
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
    /// Ordered by path, so the files under a folder are adjacent.
    files: Vec<MediaFile>,
    paths: HashSet<PathBuf>,
    claimed: HashSet<PathBuf>,
}

impl Known {
    fn under(&self, folder: &Path) -> &[MediaFile] {
        let start = self.files.partition_point(|file| file.path.as_path() < folder);
        let len = self.files[start..].partition_point(|file| file.path.starts_with(folder));
        &self.files[start..start + len]
    }
}

impl Scanner {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        clock: Arc<dyn Clock>,
        events: Publisher,
    ) -> Self {
        Self { repo, catalog, fs, lock, clock, events }
    }

    /// Links new files in the folder of every library item, sends the rest to review and forgets
    /// linked files that are gone (FR-8.2, FR-8.3, FR-8.7). Nothing else in a root folder is read.
    /// Each item folder is committed on its own; a root folder that cannot be read stops the scan
    /// before anything in it changes.
    #[instrument(skip_all)]
    pub async fn scan(&self) -> Result<ScanReport, MediaError> {
        let _lock = self.lock.acquire().await?;
        let series = self.catalog.all_series().await?;
        let movies = self.catalog.all_movies().await?;
        let known = self.known().await?;

        let items = series
            .iter()
            .map(|series| (&series.folder, MatchScope::Series(series)))
            .chain(movies.iter().map(|movie| (&movie.folder, MatchScope::Movie(movie))));
        let mut report = ScanReport::default();
        for (folder, scope) in items {
            self.scan_folder(folder, scope, &known, &mut report).await?;
        }
        Ok(report)
    }

    /// Scans the folder of one item, as `scan` does (FR-8.8); an item no longer in the library is
    /// left alone.
    #[instrument(skip_all, fields(?item))]
    pub async fn scan_item(&self, item: ItemId) -> Result<ScanReport, MediaError> {
        let _lock = self.lock.acquire().await?;
        let known = self.known().await?;
        let mut report = ScanReport::default();
        match item {
            ItemId::Series(id) => {
                if let Some(series) = self.catalog.series(id).await? {
                    self.scan_folder(&series.folder, MatchScope::Series(&series), &known, &mut report).await?;
                }
            },
            ItemId::Movie(id) => {
                if let Some(movie) = self.catalog.movie(id).await? {
                    self.scan_folder(&movie.folder, MatchScope::Movie(&movie), &known, &mut report).await?;
                }
            },
        }
        Ok(report)
    }

    /// A missing item folder holds no files; a missing root folder fails.
    async fn scan_folder(
        &self,
        folder: &ItemFolder,
        scope: MatchScope<'_>,
        known: &Known,
        report: &mut ScanReport,
    ) -> Result<(), MediaError> {
        if !self.fs.is_dir(&folder.root).await? {
            return Err(MediaError::NotAFolder(folder.root.clone()));
        }
        let path = folder.path();
        let listed = if self.fs.is_dir(&path).await? { self.fs.files(&path).await? } else { Vec::new() };
        let now = self.clock.now().timestamp();
        let (changes, events) = scan_folder(&folder.root, &path, listed, known, scope, now);
        if changes.added_files.is_empty() && changes.removed_files.is_empty() && changes.imports.is_empty() {
            return Ok(());
        }

        self.repo.save(&changes).await?;
        self.events.publish_all(events).await;
        info!(
            folder = %path.display(),
            found = changes.added_files.len(),
            vanished = changes.removed_files.len(),
            needs_review = changes.imports.len(),
            "item folder changed outside the app"
        );
        report.found += changes.added_files.len();
        report.vanished += changes.removed_files.len();
        report.needs_review.extend(changes.imports.iter().map(|import| import.id));
        Ok(())
    }

    /// Moves each file to the episodes now holding it; a file whose episodes split leaves the
    /// library and goes to review. Files and series no longer in the library are skipped.
    #[instrument(skip_all, fields(series = %event.series))]
    async fn retarget(&self, event: &EpisodesRenumbered) -> Result<(), MediaError> {
        let _lock = self.lock.acquire().await?;
        let Some(series) = self.catalog.series(event.series).await? else { return Ok(()) };
        let files: HashMap<MediaFileId, MediaFile> =
            self.repo.files().await?.into_iter().map(|file| (file.id, file)).collect();

        let mut changes = Changes::default();
        let mut rows = Vec::new();
        for renumbered in &event.files {
            let Some(file) = files.get(&renumbered.file) else { continue };
            match renumbered.span {
                Some(span) => {
                    changes.retargeted_files.push((file.id, FileTarget::Episodes { series: series.id, span }));
                },
                None => {
                    changes.removed_files.push(file.id);
                    rows.push(ImportRow {
                        path: file.path.clone(),
                        size: file.size,
                        target: None,
                        confidence: Confidence::Unknown,
                        skipped: false,
                        replace: false,
                    });
                },
            }
        }
        let mut events: Vec<Event> = Vec::new();
        if !rows.is_empty() {
            rows.sort_by(|a, b| a.path.cmp(&b.path));
            let import = Import {
                id: ImportId::generate(),
                source: series.folder.path(),
                download: None,
                status: ImportStatus::NeedsReview,
                error: None,
                rows,
                created_at: self.clock.now().timestamp(),
            };
            events.push(ImportNeedsReview { import: import.id, source: import.source.clone() }.into());
            changes.imports.push(import);
        }
        self.repo.save(&changes).await?;
        self.events.publish_all(events).await;
        info!(retargeted = changes.retargeted_files.len(), to_review = changes.removed_files.len(), "files renumbered");
        Ok(())
    }

    async fn known(&self) -> Result<Known, MediaError> {
        let mut files = self.repo.files().await?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let paths = files.iter().map(|file| file.path.clone()).collect();
        let claimed = self.repo.claimed_paths().await?.into_iter().collect();
        Ok(Known { files, paths, claimed })
    }
}

#[async_trait]
impl Handler<SeriesAdded> for Scanner {
    async fn handle(&self, event: &SeriesAdded) -> Result<(), HandlerError> {
        self.scan_item(ItemId::Series(event.series)).await?;
        Ok(())
    }
}

#[async_trait]
impl Handler<MovieAdded> for Scanner {
    async fn handle(&self, event: &MovieAdded) -> Result<(), HandlerError> {
        self.scan_item(ItemId::Movie(event.movie)).await?;
        Ok(())
    }
}

#[async_trait]
impl Handler<EpisodesRenumbered> for Scanner {
    async fn handle(&self, event: &EpisodesRenumbered) -> Result<(), HandlerError> {
        self.retarget(event).await?;
        Ok(())
    }
}

/// Detection sees paths relative to `root`.
fn scan_folder(
    root: &Path,
    folder: &Path,
    listed: Vec<ListedFile>,
    known: &Known,
    scope: MatchScope<'_>,
    now: Timestamp,
) -> (Changes, Vec<Event>) {
    let listed_paths: HashSet<&Path> = listed.iter().map(|file| file.path.as_path()).collect();
    let vanished: Vec<&MediaFile> =
        known.under(folder).iter().filter(|file| !listed_paths.contains(file.path.as_path())).collect();

    let new_files: Vec<ListedFile> = listed
        .into_iter()
        .filter(|file| !known.paths.contains(&file.path) && !known.claimed.contains(&file.path))
        .filter_map(|file| Some(ListedFile { path: file.path.strip_prefix(root).ok()?.to_owned(), size: file.size }))
        .collect();
    let mut occupied: Vec<FileTarget> = if new_files.is_empty() {
        Vec::new()
    } else {
        let gone: HashSet<MediaFileId> = vanished.iter().map(|file| file.id).collect();
        known.files.iter().filter(|file| !gone.contains(&file.id)).map(|file| file.target).collect()
    };

    let mut changes = Changes::default();
    let mut rows = Vec::new();
    for row in ImportPlan::new(&new_files, scope).rows {
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
        .map(|file| {
            FileDeleted { file: file.id, path: file.path.clone(), target: file.target, reason: DeleteReason::External }
                .into()
        })
        .collect();
    if !changes.added_files.is_empty() {
        events.push(FilesFound { files: changes.added_files.iter().map(MediaFile::linked).collect() }.into());
    }
    events.extend(
        changes
            .imports
            .iter()
            .map(|import| ImportNeedsReview { import: import.id, source: import.source.clone() }.into()),
    );
    changes.removed_files = vanished.iter().map(|file| file.id).collect();
    (changes, events)
}

#[cfg(test)]
mod tests {
    use yokoku_domain::MovieId;

    use super::*;

    #[test]
    fn under_holds_only_the_files_inside_the_folder() {
        let file = |path: &str| MediaFile {
            id: MediaFileId::generate(),
            path: path.into(),
            size: 1,
            target: FileTarget::Movie(MovieId::generate()),
            added_at: Timestamp::UNIX_EPOCH,
        };
        let mut files: Vec<MediaFile> =
            ["/tv/A (2023) Extra/a.mkv", "/tv/A (2023)/S1/b.mkv", "/tv/A (2023)/a.mkv", "/tv/A/a.mkv"]
                .into_iter()
                .map(file)
                .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let known = Known { files, paths: HashSet::new(), claimed: HashSet::new() };

        let under: Vec<_> = known.under(Path::new("/tv/A (2023)")).iter().map(|file| file.path.clone()).collect();

        assert_eq!(under, [PathBuf::from("/tv/A (2023)/S1/b.mkv"), PathBuf::from("/tv/A (2023)/a.mkv")]);
    }
}
