use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use tracing::{info, warn};
use yokoku_detect::{Classified, DownloadFile};
use yokoku_domain::{Clock, FileTarget, ImportId, MediaFileId};
use yokoku_events::{DeleteReason, Event};
use yokoku_naming::{Naming, subtitle_path};

use crate::{
    Import, ImportStatus, MediaError, MediaFile, files,
    ports::{Catalog, Changes, FileSystem, FsError, LibraryLock, MediaRepo},
};

/// How files reach the library (FR-3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportMode {
    /// The torrent keeps seeding; falls back to a copy across file systems.
    #[default]
    HardLink,
    Copy,
    Move,
}

/// Places the files of approved imports in the library (FR-5).
pub struct Importer {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
    clock: Arc<dyn Clock>,
    naming: Naming,
    mode: ImportMode,
}

/// What an import changed in the library.
#[derive(Default)]
struct Placed {
    added: Vec<MediaFile>,
    replaced: Vec<MediaFile>,
}

impl Importer {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        clock: Arc<dyn Clock>,
        naming: Naming,
        mode: ImportMode,
    ) -> Self {
        Self { repo, catalog, fs, lock, clock, naming, mode }
    }

    /// Imports that are approved, running or failed, oldest first.
    pub async fn list(&self) -> Result<Vec<Import>, MediaError> {
        let mut imports = Vec::new();
        for status in [ImportStatus::Approved, ImportStatus::Importing, ImportStatus::Failed] {
            imports.extend(self.repo.imports(status).await?);
        }
        imports.sort_by_key(|import| (import.created_at, import.id));
        Ok(imports)
    }

    /// Carries out approved imports one at a time until none is left; each is claimed first, so
    /// concurrent runners never share one. Returns each import with the status it ended in.
    ///
    /// Imports run only under the library lock, so one still `Importing` once the lock is held was
    /// left by a stopped process and is queued again.
    pub async fn run_pending(&self) -> Result<Vec<Import>, MediaError> {
        let mut finished = Vec::new();
        loop {
            let _lock = self.lock.acquire().await?;
            let recovered = self.repo.reset_importing().await?;
            if recovered > 0 {
                info!(recovered, "queued interrupted imports again");
            }
            let Some(import) = self.repo.claim_next_approved().await? else { break };
            finished.push(self.execute(import).await?);
        }
        Ok(finished)
    }

    /// Queues a failed import again.
    pub async fn retry(&self, id: ImportId) -> Result<(), MediaError> {
        let mut import = self.repo.import(id).await?.ok_or(MediaError::ImportNotFound(id))?;
        if import.status != ImportStatus::Failed {
            return Err(MediaError::NotFailed(id));
        }
        import.status = ImportStatus::Approved;
        import.error = None;
        Ok(self.repo.save(&Changes { imports: vec![import], ..Changes::default() }, &[]).await?)
    }

    /// Stores what the import changed on disk, whether it finished or failed part way.
    async fn execute(&self, mut import: Import) -> Result<Import, MediaError> {
        let mut placed = Placed::default();
        let outcome = self.place_all(&import, &mut placed).await;

        let mut events: Vec<Event> = placed
            .replaced
            .iter()
            .map(|file| Event::FileDeleted {
                file: file.id,
                path: file.path.clone(),
                target: file.target,
                reason: DeleteReason::Replaced,
            })
            .collect();
        if outcome.is_ok() || !placed.added.is_empty() {
            events.push(Event::FilesImported {
                import: import.id,
                download: import.download,
                files: placed.added.iter().map(MediaFile::linked).collect(),
            });
        }
        match outcome {
            Ok(()) => import.status = ImportStatus::Done,
            Err(error) => {
                warn!(import = %import.id, %error, "import failed");
                let reason = error.to_string();
                import.status = ImportStatus::Failed;
                import.error = Some(reason.clone());
                events.push(Event::ImportFailed { import: import.id, source: import.source.clone(), reason });
            },
        }
        let changes = Changes {
            removed_files: placed.replaced.iter().map(|file| file.id).collect(),
            added_files: placed.added,
            imports: vec![import.clone()],
            ..Changes::default()
        };
        self.repo.save(&changes, &events).await?;
        Ok(import)
    }

    /// Places every row that is not skipped, collecting each change in `placed` as it happens; the
    /// error says why the import stopped. A row already in the library as its target counts as done.
    async fn place_all(&self, import: &Import, placed: &mut Placed) -> Result<(), MediaError> {
        let library = self.repo.files().await?;
        let subtitles = self.subtitles(&import.source).await?;
        let now = self.clock.now().timestamp();

        for (number, row) in (1..).zip(&import.rows).filter(|(_, row)| !row.skipped) {
            let target = row.target.ok_or(MediaError::RowUnmatched(number))?;
            let destination = self.destination(target, &row.path).await?;
            let linked = library.iter().any(|file| file.path == destination && file.target == target);
            if linked && self.already_placed(&row.path, &destination).await? {
                continue;
            }

            if row.replace {
                for old in library.iter().filter(|file| file.target.overlaps(&target)) {
                    let placed_over = old.path == destination && self.already_placed(&row.path, &destination).await?;
                    if !placed_over {
                        self.fs.remove_file(&old.path).await?;
                    }
                    placed.replaced.push(old.clone());
                }
            }
            self.place(&row.path, &destination).await?;
            for (subtitle, tags) in subtitles.get(&row.path).into_iter().flatten() {
                let extension = subtitle.extension().unwrap_or_default().to_string_lossy();
                self.place(subtitle, &subtitle_path(&destination, tags, &extension)).await?;
            }
            placed.added.push(MediaFile {
                id: MediaFileId::generate(),
                path: destination,
                size: row.size,
                target,
                added_at: now,
            });
        }
        Ok(())
    }

    /// The naming path in the item's folder.
    async fn destination(&self, target: FileTarget, video: &Path) -> Result<PathBuf, MediaError> {
        let extension = video.extension().unwrap_or_default().to_string_lossy();
        match target {
            FileTarget::Episodes { series: id, span } => {
                let series = self.catalog.series(id).await?.ok_or(MediaError::SeriesNotFound(id))?;
                Ok(series.folder.path().join(self.naming.episode_path(&series, span, &extension)?))
            },
            FileTarget::Movie(id) => {
                let movie = self.catalog.movie(id).await?.ok_or(MediaError::MovieNotFound(id))?;
                Ok(movie.folder.path().join(self.naming.movie_path(&movie, &extension)))
            },
        }
    }

    /// Subtitles of each video in the download.
    async fn subtitles(
        &self,
        source: &Path,
    ) -> Result<HashMap<PathBuf, Vec<(PathBuf, yokoku_domain::SubtitleTags)>>, FsError> {
        let files: Vec<DownloadFile> = if self.fs.is_dir(source).await? {
            self.fs.files(source).await?
        } else {
            self.fs.files_in(source.parent().unwrap_or(source)).await?
        };
        Ok(Classified::from_files(&files)
            .videos
            .into_iter()
            .map(|video| {
                (video.path, video.subtitles.into_iter().map(|subtitle| (subtitle.path, subtitle.tags)).collect())
            })
            .collect())
    }

    /// The destination already holds the source: the same data or size, or the moved file itself.
    async fn already_placed(&self, source: &Path, destination: &Path) -> Result<bool, MediaError> {
        let from = self.fs.stat(source).await?;
        Ok(match (from, self.fs.stat(destination).await?) {
            (Some(from), Some(to)) => from.same_file(&to) || from.size == to.size,
            (None, Some(_)) => self.mode == ImportMode::Move,
            (_, None) => false,
        })
    }

    /// A destination that already holds the file counts as placed, so a retry picks up where an
    /// interrupted import stopped.
    async fn place(&self, source: &Path, destination: &Path) -> Result<(), MediaError> {
        if self.already_placed(source, destination).await? {
            return Ok(());
        }
        let from = self.fs.stat(source).await?;
        match (from, self.fs.stat(destination).await?) {
            (_, Some(_)) => return Err(MediaError::AlreadyExists(destination.to_owned())),
            (None, None) => return Err(MediaError::SourceMissing(source.to_owned())),
            (Some(_), None) => {},
        }

        let result = match self.mode {
            ImportMode::HardLink => match self.fs.hard_link(source, destination).await {
                Err(error) if error.source.kind() == io::ErrorKind::CrossesDevices => {
                    warn!(path = %source.display(), "cannot hard-link across file systems; copying");
                    self.fs.copy(source, destination).await
                },
                result => result,
            },
            ImportMode::Copy => self.fs.copy(source, destination).await,
            ImportMode::Move => files::move_file(self.fs.as_ref(), source, destination).await,
        };
        Ok(result?)
    }
}
