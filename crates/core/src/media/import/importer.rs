use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};
use yokoku_domain::{
    Clock, FileTarget, ImportId, Live, MediaFileId,
    events::{DeleteReason, Event, FileDeleted, FilesImported, ImportFailed},
    naming::{Naming, subtitle_path},
};

use crate::{
    events::{Publisher, QueueChanges},
    media::{
        Import, ImportRow, ImportStatus, MediaError, MediaFile, Resolution,
        detect::{Classified, ListedFile},
        ports::{Catalog, Changes, FileSystem, FsError, LibraryLock, MediaRepo},
    },
};

/// How files reach the library (FR-3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportMode {
    /// The torrent keeps seeding; falls back to a copy across file systems.
    #[default]
    #[serde(rename = "hardlink")]
    HardLink,
    Copy,
    Move,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct ImportSettings {
    pub mode: ImportMode,
}

/// Places the files of approved imports in the library (FR-5).
pub struct Importer {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
    clock: Arc<dyn Clock>,
    naming: Live<Naming>,
    mode: Live<ImportMode>,
    events: Publisher,
    changes: QueueChanges,
}

/// Where a matched file goes in the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    /// The item's folder.
    pub folder: PathBuf,
    /// Relative to `folder`, from the naming templates.
    pub name: PathBuf,
}

impl Destination {
    pub fn path(&self) -> PathBuf {
        self.folder.join(&self.name)
    }
}

/// What an import changed in the library.
#[derive(Default)]
struct Placed {
    added: Vec<MediaFile>,
    replaced: Vec<MediaFile>,
}

impl Importer {
    #[expect(clippy::too_many_arguments, reason = "one argument per dependency")]
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        clock: Arc<dyn Clock>,
        naming: Live<Naming>,
        mode: Live<ImportMode>,
        events: Publisher,
        changes: QueueChanges,
    ) -> Self {
        Self { repo, catalog, fs, lock, clock, naming, mode, events, changes }
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
                self.changes.notify();
                info!(recovered, "queued interrupted imports again");
            }
            let Some(import) = self.repo.claim_next_approved().await? else { break };
            self.changes.notify();
            finished.push(self.execute(import).await?);
        }
        Ok(finished)
    }

    /// Queues a failed import again.
    #[instrument(skip_all, fields(import = %id))]
    pub async fn retry(&self, id: ImportId) -> Result<(), MediaError> {
        let mut import = self.repo.import(id).await?.ok_or(MediaError::ImportNotFound(id))?;
        if import.status != ImportStatus::Failed {
            return Err(MediaError::NotFailed(id));
        }
        import.status = ImportStatus::Approved;
        import.error = None;
        self.repo.save(&Changes { imports: vec![import], ..Changes::default() }).await?;
        self.changes.notify();
        info!("import queued again");
        Ok(())
    }

    /// Stores what the import changed on disk, whether it finished or failed part way.
    #[instrument(skip_all, fields(import = %import.id, source = %import.source.display()))]
    async fn execute(&self, mut import: Import) -> Result<Import, MediaError> {
        let mut placed = Placed::default();
        let outcome = self.place_all(&import, &mut placed).await;

        let mut events: Vec<Event> = placed
            .replaced
            .iter()
            .map(|file| {
                FileDeleted {
                    file: file.id,
                    path: file.path.clone(),
                    target: file.target,
                    reason: DeleteReason::Replaced,
                }
                .into()
            })
            .collect();
        if outcome.is_ok() || !placed.added.is_empty() {
            events.push(
                FilesImported {
                    import: import.id,
                    download: import.download,
                    files: placed.added.iter().map(MediaFile::linked).collect(),
                }
                .into(),
            );
        }
        match outcome {
            Ok(()) => {
                import.status = ImportStatus::Done;
                info!(placed = placed.added.len(), replaced = placed.replaced.len(), "import done");
            },
            Err(error) => {
                warn!(%error, placed = placed.added.len(), replaced = placed.replaced.len(), "import failed");
                let reason = error.to_string();
                import.status = ImportStatus::Failed;
                import.error = Some(reason.clone());
                events.push(ImportFailed { import: import.id, source: import.source.clone(), reason }.into());
            },
        }
        let changes = Changes {
            removed_files: placed.replaced.iter().map(|file| file.id).collect(),
            added_files: placed.added,
            imports: vec![import.clone()],
            ..Changes::default()
        };
        self.repo.save(&changes).await?;
        self.changes.notify();
        self.events.publish_all(events).await;
        Ok(import)
    }

    /// Places every row that is not skipped, collecting each change in `placed` as it happens; the
    /// error says why the import stopped. A row already in the library as its target counts as done.
    async fn place_all(&self, import: &Import, placed: &mut Placed) -> Result<(), MediaError> {
        let library = self.repo.files().await?;
        let subtitles = self.subtitles(&import.source).await?;
        let destinations = self.destinations(&import.rows).await?;
        let now = self.clock.now().timestamp();

        for ((number, row), destination) in
            (1..).zip(&import.rows).zip(destinations).filter(|((_, row), _)| !row.skipped)
        {
            let target = row.target().ok_or(MediaError::RowUnmatched(number))?;
            let destination = destination.ok_or(MediaError::RowUnmatched(number))?.path();
            let linked = library.iter().any(|file| file.path == destination && file.target == target);
            if linked && self.already_placed(&row.path, &destination).await? {
                debug!(path = %destination.display(), "already in the library");
                continue;
            }

            let olds: Vec<&MediaFile> = library
                .iter()
                .filter(|file| row.resolution == Resolution::Replace && file.target.overlaps(&target))
                .collect();
            let occupied =
                olds.iter().any(|old| old.path == destination) && self.fs.stat(&destination).await?.is_some();
            let aside = if occupied && !self.already_placed(&row.path, &destination).await? {
                Some(self.set_aside(&destination).await?)
            } else {
                None
            };
            if let Err(error) = self.place(&row.path, &destination).await {
                if let Some(aside) = &aside {
                    self.fs.rename(aside, &destination).await?;
                }
                return Err(error);
            }
            for old in olds {
                let path = if old.path == destination { aside.as_ref() } else { Some(&old.path) };
                if let Some(path) = path {
                    self.fs.remove_file(path).await?;
                    debug!(path = %old.path.display(), "removed the file being replaced");
                }
                placed.replaced.push(old.clone());
            }
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

    /// Where each row would be placed, in row order; `None` for a skipped or unmatched row. A row
    /// kept beside others takes the first free numbered name after the other rows took theirs.
    pub async fn destinations(&self, rows: &[ImportRow]) -> Result<Vec<Option<Destination>>, MediaError> {
        let mut destinations = Vec::with_capacity(rows.len());
        for row in rows {
            destinations.push(match row.target().filter(|_| !row.skipped) {
                Some(target) => Some(self.destination(target, &row.path).await?),
                None => None,
            });
        }

        let kept = |row: &ImportRow| row.resolution == Resolution::KeepBoth;
        let library: HashSet<PathBuf> = self.repo.files().await?.into_iter().map(|file| file.path).collect();
        let mut taken: HashSet<PathBuf> = rows
            .iter()
            .zip(&destinations)
            .filter(|(row, _)| !kept(row))
            .filter_map(|(_, destination)| destination.as_ref().map(Destination::path))
            .collect();
        for (row, destination) in rows.iter().zip(&mut destinations) {
            let Some(destination) = destination.as_mut().filter(|_| kept(row)) else { continue };
            destination.name = self.free_name(&row.path, destination, &library, &taken).await?;
            taken.insert(destination.path());
        }
        Ok(destinations)
    }

    /// The destination's name or the first of `name (2)`, `name (3)`, … that already holds
    /// `source`, or holds nothing: no file on disk, in the library, or in `taken`. Ends because
    /// only finitely many of those names are occupied.
    async fn free_name(
        &self,
        source: &Path,
        destination: &Destination,
        library: &HashSet<PathBuf>,
        taken: &HashSet<PathBuf>,
    ) -> Result<PathBuf, MediaError> {
        let mut number = 1;
        loop {
            let name = numbered(&destination.name, number);
            let path = destination.folder.join(&name);
            if !taken.contains(&path)
                && (self.already_placed(source, &path).await?
                    || (!library.contains(&path) && self.fs.stat(&path).await?.is_none()))
            {
                return Ok(name);
            }
            number += 1;
        }
    }

    /// The naming path in the item's folder.
    async fn destination(&self, target: FileTarget, video: &Path) -> Result<Destination, MediaError> {
        let extension = video.extension().unwrap_or_default().to_string_lossy();
        match target {
            FileTarget::Episodes { series: id, span } => {
                let series = self.catalog.series(id).await?.ok_or(MediaError::SeriesNotFound(id))?;
                let name = self.naming.current().episode_path(&series, span, &extension)?;
                Ok(Destination { folder: series.folder.path(), name })
            },
            FileTarget::Movie(id) => {
                let movie = self.catalog.movie(id).await?.ok_or(MediaError::MovieNotFound(id))?;
                let name = self.naming.current().movie_path(&movie, &extension);
                Ok(Destination { folder: movie.folder.path(), name })
            },
        }
    }

    /// Subtitles of each video in the download.
    async fn subtitles(
        &self,
        source: &Path,
    ) -> Result<HashMap<PathBuf, Vec<(PathBuf, yokoku_domain::SubtitleTags)>>, FsError> {
        let files: Vec<ListedFile> = if self.fs.is_dir(source).await? {
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

    /// Moves the file at `path` to a hidden `.<name>.replaced` beside it and returns that path.
    async fn set_aside(&self, path: &Path) -> Result<PathBuf, MediaError> {
        let aside =
            path.with_file_name(format!(".{}.replaced", path.file_name().unwrap_or_default().to_string_lossy()));
        self.fs.rename(path, &aside).await?;
        Ok(aside)
    }

    /// The destination already holds the source: the same data or bytes, or the moved file itself.
    async fn already_placed(&self, source: &Path, destination: &Path) -> Result<bool, MediaError> {
        let from = self.fs.stat(source).await?;
        Ok(match (from, self.fs.stat(destination).await?) {
            (Some(from), Some(to)) => {
                from.same_file(&to) || (from.size == to.size && self.fs.same_contents(source, destination).await?)
            },
            (None, Some(_)) => self.mode.current() == ImportMode::Move,
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

        let mode = self.mode.current();
        let result = match mode {
            ImportMode::HardLink => match self.fs.hard_link(source, destination).await {
                Err(error) if error.source.kind() == io::ErrorKind::CrossesDevices => {
                    warn!(path = %source.display(), "cannot hard-link across file systems; copying");
                    self.fs.copy(source, destination).await
                },
                result => result,
            },
            ImportMode::Copy => self.fs.copy(source, destination).await,
            ImportMode::Move => self.fs.move_file(source, destination).await,
        };
        result?;
        debug!(from = %source.display(), to = %destination.display(), ?mode, "placed");
        Ok(())
    }
}

/// `name` for 1, else `name (number)` before its extension.
fn numbered(name: &Path, number: u32) -> PathBuf {
    if number == 1 {
        return name.to_owned();
    }
    let stem = name.file_stem().unwrap_or_default().to_string_lossy();
    let file = match name.extension() {
        Some(extension) => format!("{stem} ({number}).{}", extension.to_string_lossy()),
        None => format!("{stem} ({number})"),
    };
    name.with_file_name(file)
}
