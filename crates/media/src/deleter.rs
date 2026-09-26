use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{FileTarget, ItemId};
use yokoku_events::{DeleteReason, Event, HandlerError, Recorded, Subscriber};

use crate::{
    MediaError, MediaFile, files,
    ports::{Changes, FileSystem, LibraryLock, MediaRepo},
};

/// Deletes library files (FR-8.4).
pub struct Deleter {
    repo: Arc<dyn MediaRepo>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
}

impl Deleter {
    pub fn new(repo: Arc<dyn MediaRepo>, fs: Arc<dyn FileSystem>, lock: Arc<dyn LibraryLock>) -> Self {
        Self { repo, fs, lock }
    }

    /// The library files holding any of `target`.
    pub async fn files_of(&self, target: FileTarget) -> Result<Vec<MediaFile>, MediaError> {
        Ok(self.repo.files().await?.into_iter().filter(|file| file.target.overlaps(&target)).collect())
    }

    /// The library files of `item`.
    pub async fn files_of_item(&self, item: ItemId) -> Result<Vec<MediaFile>, MediaError> {
        Ok(self.repo.files().await?.into_iter().filter(|file| file.target.item() == item).collect())
    }

    /// Removes the library files holding any of `target`, with their subtitles.
    pub async fn delete(&self, target: FileTarget) -> Result<Vec<MediaFile>, MediaError> {
        let _lock = self.lock.acquire().await?;
        let files = self.files_of(target).await?;
        if files.is_empty() {
            return Err(MediaError::NoFile);
        }
        self.remove(files, DeleteReason::User).await
    }

    /// Each file is removed and committed on its own, so storage matches the disk if one fails.
    async fn remove(&self, files: Vec<MediaFile>, reason: DeleteReason) -> Result<Vec<MediaFile>, MediaError> {
        let roots = self.repo.root_folders().await?;
        for file in &files {
            let root = roots.iter().map(|root| root.path.as_path()).find(|root| file.path.starts_with(root));
            let subtitles = files::sidecar_subtitles(self.fs.as_ref(), &file.path).await?;
            let paths = std::iter::once(file.path.clone()).chain(subtitles.into_iter().map(|subtitle| subtitle.path));
            for path in paths {
                self.fs.remove_file(&path).await?;
            }
            if let (Some(root), Some(folder)) = (root, file.path.parent()) {
                self.fs.remove_empty_folders(folder, root).await?;
            }

            let event = Event::FileDeleted { file: file.id, path: file.path.clone(), target: file.target, reason };
            self.repo.save(&Changes { removed_files: vec![file.id], ..Changes::default() }, &[event]).await?;
        }
        Ok(files)
    }

    async fn remove_item(&self, item: ItemId) -> Result<(), MediaError> {
        let _lock = self.lock.acquire().await?;
        let files = self.files_of_item(item).await?;
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
