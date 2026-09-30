use std::{
    collections::{HashMap, hash_map::Entry},
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use tracing::{info, instrument, warn};
use yokoku_domain::{
    FileTarget, Live, MediaFileId, MovieId, Series, SeriesId,
    naming::{Naming, subtitle_path},
};

use crate::{
    events::{FileRenamed, Publisher},
    media::{
        MediaError, MediaFile, files,
        ports::{Catalog, Changes, FileSystem, LibraryLock, MediaRepo},
    },
};

/// Moves library files to the paths naming gives them (FR-5.7).
pub struct Renamer {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
    naming: Live<Naming>,
    events: Publisher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameScope {
    All,
    Series(SeriesId),
    Movie(MovieId),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenamePlan {
    /// Ordered by current path.
    pub renames: Vec<Rename>,
    pub skipped: Vec<Skipped>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub file: MediaFileId,
    pub target: FileTarget,
    pub root: PathBuf,
    pub video: Move,
    /// Subtitles next to the video, named after it.
    pub subtitles: Vec<Move>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: PathBuf,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The file is in no root folder.
    OutsideRoots,
    /// Its series, episodes or movie are no longer in the library.
    NotInLibrary,
    /// Another file in the plan would get the same path.
    SharedTarget,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutsideRoots => "not in a root folder",
            Self::NotInLibrary => "its item is no longer in the library",
            Self::SharedTarget => "another file would get the same name",
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RenameReport {
    pub renamed: Vec<Rename>,
    pub skipped: Vec<Skipped>,
    pub failed: Vec<RenameFailure>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RenameFailure {
    pub path: PathBuf,
    pub error: String,
}

impl Renamer {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        naming: Live<Naming>,
        events: Publisher,
    ) -> Self {
        Self { repo, catalog, fs, lock, naming, events }
    }

    /// The moves `apply` would make; nothing on disk changes.
    pub async fn preview(&self, scope: RenameScope) -> Result<RenamePlan, MediaError> {
        let roots = self.repo.root_folders().await?;
        let mut series: HashMap<SeriesId, Option<Series>> = HashMap::new();
        let mut plan = RenamePlan::default();

        for file in self.repo.files().await?.into_iter().filter(|file| scope.contains(file.target)) {
            let skip = |reason| Skipped { path: file.path.clone(), reason };
            let Some(root) = roots.iter().find(|root| file.path.starts_with(&root.path)) else {
                plan.skipped.push(skip(SkipReason::OutsideRoots));
                continue;
            };
            let Some(to) = self.target_path(&file, &mut series).await? else {
                plan.skipped.push(skip(SkipReason::NotInLibrary));
                continue;
            };
            let rename = Rename {
                file: file.id,
                target: file.target,
                root: root.path.clone(),
                subtitles: self.subtitle_moves(&file.path, &to).await?,
                video: Move { from: file.path, to },
            };
            if rename.moves().any(|step| step.from != step.to) {
                plan.renames.push(rename);
            }
        }

        let shared = shared_targets(&plan.renames);
        let (renames, clashing): (Vec<_>, Vec<_>) =
            plan.renames.into_iter().partition(|rename| rename.moves().all(|step| !shared.contains(&step.to)));
        plan.renames = renames;
        plan.skipped.extend(
            clashing.into_iter().map(|rename| Skipped { path: rename.video.from, reason: SkipReason::SharedTarget }),
        );
        Ok(plan)
    }

    /// Carries out the plan file by file, for the files in `only` or for all of them; a file that
    /// cannot be moved is reported and left alone, and an old folder that cannot be removed is
    /// only logged.
    #[instrument(skip_all, fields(?scope))]
    pub async fn apply(&self, scope: RenameScope, only: Option<&[MediaFileId]>) -> Result<RenameReport, MediaError> {
        let _lock = self.lock.acquire().await?;
        let plan = self.preview(scope).await?;
        let mut report = RenameReport { skipped: plan.skipped, ..RenameReport::default() };

        for rename in plan.renames.into_iter().filter(|rename| only.is_none_or(|files| files.contains(&rename.file))) {
            let video = &rename.video;
            if video.from != video.to {
                if let Err(error) = self.fs.rename(&video.from, &video.to).await {
                    report.failed.push(RenameFailure { path: video.from.clone(), error: error.to_string() });
                    continue;
                }
                let changes = Changes { renamed_files: vec![(rename.file, video.to.clone())], ..Changes::default() };
                let event = FileRenamed {
                    file: rename.file,
                    from: video.from.clone(),
                    to: video.to.clone(),
                    target: Some(rename.target),
                };
                if let Err(error) = self.repo.save(&changes).await {
                    if let Err(undo) = self.fs.rename(&video.to, &video.from).await {
                        warn!(path = %video.to.display(), %undo, "could not move a file back after a failed save");
                    }
                    return Err(error.into());
                }
                self.events.publish(event).await;
                info!(from = %video.from.display(), to = %video.to.display(), "file renamed");
            }
            for subtitle in rename.subtitles.iter().filter(|subtitle| subtitle.from != subtitle.to) {
                if let Err(error) = self.fs.rename(&subtitle.from, &subtitle.to).await {
                    report.failed.push(RenameFailure { path: subtitle.from.clone(), error: error.to_string() });
                }
            }
            if let Some(old_folder) = video.from.parent()
                && let Err(error) = self.fs.remove_empty_folders(old_folder, &rename.root).await
            {
                warn!(%error, folder = %old_folder.display(), "could not remove the old folder of a renamed file");
            }
            report.renamed.push(rename);
        }
        Ok(report)
    }

    /// The naming path in the item's folder; `None` when the file's series, episodes or movie are gone.
    async fn target_path(
        &self,
        file: &MediaFile,
        series: &mut HashMap<SeriesId, Option<Series>>,
    ) -> Result<Option<PathBuf>, MediaError> {
        let extension = file.path.extension().unwrap_or_default().to_string_lossy();
        Ok(match file.target {
            FileTarget::Episodes { series: id, span } => {
                let series = match series.entry(id) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(self.catalog.series(id).await?),
                };
                series.as_ref().and_then(|series| {
                    Some(series.folder.path().join(self.naming.current().episode_path(series, span, &extension).ok()?))
                })
            },
            FileTarget::Movie(id) => self
                .catalog
                .movie(id)
                .await?
                .map(|movie| movie.folder.path().join(self.naming.current().movie_path(&movie, &extension))),
        })
    }

    /// Subtitles beside `video` whose names start with the video's name.
    async fn subtitle_moves(&self, video: &Path, to: &Path) -> Result<Vec<Move>, MediaError> {
        let subtitles = files::sidecar_subtitles(self.fs.as_ref(), video).await?;
        Ok(subtitles
            .into_iter()
            .map(|subtitle| {
                let extension = subtitle.path.extension().unwrap_or_default().to_string_lossy().into_owned();
                Move { to: subtitle_path(to, &subtitle.tags, &extension), from: subtitle.path }
            })
            .collect())
    }
}

impl RenameScope {
    fn contains(self, target: FileTarget) -> bool {
        match (self, target) {
            (Self::All, _) => true,
            (Self::Series(id), FileTarget::Episodes { series, .. }) => id == series,
            (Self::Movie(id), FileTarget::Movie(movie)) => id == movie,
            _ => false,
        }
    }
}

impl Rename {
    fn moves(&self) -> impl Iterator<Item = &Move> {
        std::iter::once(&self.video).chain(&self.subtitles)
    }
}

/// Paths more than one move in the plan would end at.
fn shared_targets(renames: &[Rename]) -> Vec<PathBuf> {
    let mut counts: HashMap<&Path, usize> = HashMap::new();
    for step in renames.iter().flat_map(Rename::moves) {
        *counts.entry(&step.to).or_default() += 1;
    }
    counts.into_iter().filter(|&(_, count)| count > 1).map(|(path, _)| path.to_owned()).collect()
}
