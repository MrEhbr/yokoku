use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use tracing::warn;
use yokoku_detect::{DownloadFile, classify};
use yokoku_domain::{Clock, FileTarget, ImportId, MediaFileId};
use yokoku_events::{DeleteReason, Event};
use yokoku_naming::{Naming, subtitle_path};

use crate::{
    Import, ImportStatus, MediaError, MediaFile, RootFolder, RootKind,
    ports::{Catalog, Changes, FileSystem, FsError, MediaRepo},
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
        clock: Arc<dyn Clock>,
        naming: Naming,
        mode: ImportMode,
    ) -> Self {
        Self { repo, catalog, fs, clock, naming, mode }
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
    pub async fn run_pending(&self) -> Result<Vec<Import>, MediaError> {
        let mut finished = Vec::new();
        while let Some(import) = self.repo.claim_next_approved().await? {
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

    /// Queues again the imports a stopped process left running; returns how many.
    pub async fn recover(&self) -> Result<u64, MediaError> {
        Ok(self.repo.reset_importing().await?)
    }

    async fn execute(&self, mut import: Import) -> Result<Import, MediaError> {
        let (changes, events) = match self.place_all(&import).await {
            Ok(placed) => {
                import.status = ImportStatus::Done;
                let mut events: Vec<Event> = placed
                    .replaced
                    .iter()
                    .map(|file| Event::FileDeleted {
                        file: file.id,
                        path: file.path.clone(),
                        target: file.target,
                        reason: DeleteReason::Replaced,
                        recycled: false,
                    })
                    .collect();
                events.push(Event::FilesImported {
                    import: import.id,
                    files: placed.added.iter().map(MediaFile::linked).collect(),
                });
                let changes = Changes {
                    added_files: placed.added,
                    removed_files: placed.replaced.iter().map(|file| file.id).collect(),
                    imports: vec![import.clone()],
                    ..Changes::default()
                };
                (changes, events)
            },
            Err(reason) => {
                warn!(import = %import.id, %reason, "import failed");
                import.status = ImportStatus::Failed;
                import.error = Some(reason.clone());
                let event = Event::ImportFailed { import: import.id, source: import.source.clone(), reason };
                (Changes { imports: vec![import.clone()], ..Changes::default() }, vec![event])
            },
        };
        self.repo.save(&changes, &events).await?;
        Ok(import)
    }

    /// Places every row that is not skipped; the error says why the import stopped.
    async fn place_all(&self, import: &Import) -> Result<Placed, String> {
        let roots = self.repo.root_folders().await.map_err(|error| error.to_string())?;
        let library = self.repo.files().await.map_err(|error| error.to_string())?;
        let subtitles = self.subtitles(&import.source).await.map_err(|error| error.to_string())?;
        let now = self.clock.now().timestamp();
        let mut placed = Placed::default();

        for (number, row) in (1..).zip(&import.rows).filter(|(_, row)| !row.skipped) {
            let target = row.target.ok_or_else(|| format!("row {number} has no match"))?;
            let root = root_for(target, &roots, &library).ok_or_else(|| match target {
                FileTarget::Episodes { .. } => "there is no series root folder".to_owned(),
                FileTarget::Movie(_) => "there is no movie root folder".to_owned(),
            })?;
            let destination = root.join(self.relative_path(target, &row.path).await?);

            if row.replace {
                for old in library.iter().filter(|file| file.target.overlaps(&target)) {
                    let placed_over = old.path == destination && self.already_placed(&row.path, &destination).await?;
                    if !placed_over {
                        self.fs.remove_file(&old.path).await.map_err(|error| error.to_string())?;
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
        Ok(placed)
    }

    async fn relative_path(&self, target: FileTarget, video: &Path) -> Result<PathBuf, String> {
        let extension = video.extension().unwrap_or_default().to_string_lossy();
        let gone = || "its series or movie is no longer in the library".to_owned();
        match target {
            FileTarget::Episodes { series, span } => {
                let series = self.catalog.series(series).await.map_err(|error| error.to_string())?.ok_or_else(gone)?;
                self.naming.episode_path(&series, span, &extension).map_err(|error| error.to_string())
            },
            FileTarget::Movie(movie) => {
                let movie = self.catalog.movie(movie).await.map_err(|error| error.to_string())?.ok_or_else(gone)?;
                Ok(self.naming.movie_path(&movie, &extension))
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
        Ok(classify(&files)
            .videos
            .into_iter()
            .map(|video| {
                (video.path, video.subtitles.into_iter().map(|subtitle| (subtitle.path, subtitle.tags)).collect())
            })
            .collect())
    }

    /// The destination already holds the source: the same data or size, or the moved file itself.
    async fn already_placed(&self, source: &Path, destination: &Path) -> Result<bool, String> {
        let describe = |error: FsError| error.to_string();
        let from = self.fs.stat(source).await.map_err(describe)?;
        Ok(match (from, self.fs.stat(destination).await.map_err(describe)?) {
            (Some(from), Some(to)) => from.same_file(&to) || from.size == to.size,
            (None, Some(_)) => self.mode == ImportMode::Move,
            (_, None) => false,
        })
    }

    /// A destination that already holds the file counts as placed, so a retry picks up where an
    /// interrupted import stopped.
    async fn place(&self, source: &Path, destination: &Path) -> Result<(), String> {
        if self.already_placed(source, destination).await? {
            return Ok(());
        }
        let describe = |error: FsError| error.to_string();
        let from = self.fs.stat(source).await.map_err(describe)?;
        match (from, self.fs.stat(destination).await.map_err(describe)?) {
            (_, Some(_)) => return Err(format!("{} already exists", destination.display())),
            (None, None) => return Err(format!("{} is missing", source.display())),
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
            ImportMode::Move => match self.fs.rename(source, destination).await {
                Err(error) if error.source.kind() == io::ErrorKind::CrossesDevices => {
                    match self.fs.copy(source, destination).await {
                        Ok(()) => self.fs.remove_file(source).await,
                        Err(error) => Err(error),
                    }
                },
                result => result,
            },
        };
        result.map_err(describe)
    }
}

/// The root holding the series' other files, or else the first root of the item's kind.
fn root_for<'a>(target: FileTarget, roots: &'a [RootFolder], library: &[MediaFile]) -> Option<&'a Path> {
    let kind = match target {
        FileTarget::Episodes { .. } => RootKind::Series,
        FileTarget::Movie(_) => RootKind::Movies,
    };
    let candidates = || roots.iter().filter(move |root| root.kind == kind);
    let same_item = |file: &&MediaFile| match (file.target, target) {
        (FileTarget::Episodes { series, .. }, FileTarget::Episodes { series: wanted, .. }) => series == wanted,
        (file, wanted) => file == wanted,
    };
    library
        .iter()
        .filter(same_item)
        .find_map(|file| candidates().find(|root| file.path.starts_with(&root.path)))
        .or_else(|| candidates().next())
        .map(|root| root.path.as_path())
}
