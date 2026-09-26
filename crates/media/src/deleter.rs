use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use jiff::{ToSpan, civil::Date};
use yokoku_domain::{Clock, FileTarget, ItemId};
use yokoku_events::{DeleteReason, Event, HandlerError, Recorded, Subscriber};

use crate::{
    MediaError, MediaFile, files,
    ports::{Changes, FileSystem, LibraryLock, MediaRepo},
};

/// Where deleted files go instead of being removed, and for how many days (FR-8.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recycle {
    pub folder: PathBuf,
    pub keep_days: u32,
}

/// Deletes library files, or moves them to the recycle folder (FR-8.4, FR-8.5).
pub struct Deleter {
    repo: Arc<dyn MediaRepo>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
    clock: Arc<dyn Clock>,
    recycle: Option<Recycle>,
}

impl Deleter {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        clock: Arc<dyn Clock>,
        recycle: Option<Recycle>,
    ) -> Self {
        Self { repo, fs, lock, clock, recycle }
    }

    /// Removes the library files holding any of `target`, with their subtitles.
    pub async fn delete(&self, target: FileTarget) -> Result<Vec<MediaFile>, MediaError> {
        let _lock = self.lock.acquire().await?;
        let files: Vec<MediaFile> =
            self.repo.files().await?.into_iter().filter(|file| file.target.overlaps(&target)).collect();
        if files.is_empty() {
            return Err(MediaError::NoFile);
        }
        self.remove(files, DeleteReason::User).await
    }

    /// Removes the recycle folder's day folders older than `keep_days`; returns how many.
    pub async fn clean_recycle(&self) -> Result<usize, MediaError> {
        let Some(recycle) = &self.recycle else { return Ok(0) };
        if !self.fs.is_dir(&recycle.folder).await? {
            return Ok(0);
        }
        let oldest_kept = self.clock.now().date() - i64::from(recycle.keep_days).days();
        let mut removed = 0;
        for folder in self.fs.folders_in(&recycle.folder).await? {
            let day = folder.file_name().and_then(|name| name.to_str()?.parse::<Date>().ok());
            if day.is_some_and(|day| day < oldest_kept) {
                self.fs.remove_folder(&folder).await?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Each file is removed and committed on its own, so storage matches the disk if one fails.
    async fn remove(&self, files: Vec<MediaFile>, reason: DeleteReason) -> Result<Vec<MediaFile>, MediaError> {
        let roots = self.repo.root_folders().await?;
        let today = self.clock.now().date();
        for file in &files {
            let root = roots.iter().map(|root| root.path.as_path()).find(|root| file.path.starts_with(root));
            let subtitles = files::sidecar_subtitles(self.fs.as_ref(), &file.path).await?;
            let paths = std::iter::once(file.path.clone()).chain(subtitles.into_iter().map(|subtitle| subtitle.path));
            for path in paths {
                match &self.recycle {
                    Some(recycle) => {
                        let destination = self.recycled_path(recycle, today, root, file, &path).await?;
                        files::move_file(self.fs.as_ref(), &path, &destination).await?;
                    },
                    None => self.fs.remove_file(&path).await?,
                }
            }
            if let (Some(root), Some(folder)) = (root, file.path.parent()) {
                self.fs.remove_empty_folders(folder, root).await?;
            }

            let event = Event::FileDeleted {
                file: file.id,
                path: file.path.clone(),
                target: file.target,
                reason,
                recycled: self.recycle.is_some(),
            };
            self.repo.save(&Changes { removed_files: vec![file.id], ..Changes::default() }, &[event]).await?;
        }
        Ok(files)
    }

    /// `<recycle>/<today>/<root folder name>/<path under the root>`, or under the file's id when that
    /// is taken.
    async fn recycled_path(
        &self,
        recycle: &Recycle,
        today: Date,
        root: Option<&Path>,
        file: &MediaFile,
        path: &Path,
    ) -> Result<PathBuf, MediaError> {
        let relative = root
            .and_then(|root| path.strip_prefix(root.parent().unwrap_or(root)).ok())
            .unwrap_or(path.file_name().map_or(path, Path::new));
        let day = recycle.folder.join(today.to_string());
        let destination = day.join(relative);
        match self.fs.stat(&destination).await? {
            None => Ok(destination),
            Some(_) => Ok(day.join(file.id.to_string()).join(relative)),
        }
    }

    async fn remove_item(&self, item: ItemId) -> Result<(), MediaError> {
        let _lock = self.lock.acquire().await?;
        let files: Vec<MediaFile> =
            self.repo.files().await?.into_iter().filter(|file| file.target.item() == item).collect();
        if !files.is_empty() {
            self.remove(files, DeleteReason::ItemRemoved).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Subscriber for Deleter {
    fn name(&self) -> &'static str {
        "media.removals"
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        match &recorded.event {
            Event::SeriesRemoved { series, delete_files: true, .. } => {
                self.remove_item(ItemId::Series(*series)).await?
            },
            Event::MovieRemoved { movie, delete_files: true, .. } => self.remove_item(ItemId::Movie(*movie)).await?,
            _ => {},
        }
        Ok(())
    }
}
