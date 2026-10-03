use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};
use yokoku_domain::{
    ItemId, MediaFileId,
    events::{DeleteReason, FileDeleted, MovieRemoved, SeriesRemoved},
};

use crate::{
    events::{Handler, HandlerError, Publisher},
    media::{
        MediaError, MediaFile, RootFolders, files,
        ports::{Changes, FileSystem, LibraryLock, MediaRepo},
    },
};

/// Deletes library files.
pub struct Deleter {
    repo: Arc<dyn MediaRepo>,
    roots: Arc<RootFolders>,
    fs: Arc<dyn FileSystem>,
    lock: Arc<dyn LibraryLock>,
    events: Publisher,
}

impl Deleter {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        roots: Arc<RootFolders>,
        fs: Arc<dyn FileSystem>,
        lock: Arc<dyn LibraryLock>,
        events: Publisher,
    ) -> Self {
        Self { repo, roots, fs, lock, events }
    }

    /// The library files of `item`.
    pub async fn files_of_item(&self, item: ItemId) -> Result<Vec<MediaFile>, MediaError> {
        Ok(self.repo.files_of(item).await?)
    }

    /// Removes the library files of `item` among `ids`, with their subtitles; ids of other items
    /// are left alone.
    pub async fn delete_files(&self, item: ItemId, ids: &[MediaFileId]) -> Result<Vec<MediaFile>, MediaError> {
        let _lock = self.lock.acquire().await?;
        let files: Vec<_> = self.files_of_item(item).await?.into_iter().filter(|file| ids.contains(&file.id)).collect();
        if files.is_empty() {
            return Err(MediaError::NoFile);
        }
        self.remove(files, DeleteReason::User).await
    }

    /// Removes and commits each video on its own.
    /// Subtitles and emptied folders go afterwards; a failure there is only logged.
    #[instrument(skip_all, fields(?reason))]
    async fn remove(&self, files: Vec<MediaFile>, reason: DeleteReason) -> Result<Vec<MediaFile>, MediaError> {
        let roots = self.roots.list().await?;
        for file in &files {
            let subtitles = files::sidecar_subtitles(self.fs.as_ref(), &file.path).await?;
            self.fs.remove_file(&file.path).await?;
            self.repo.save(&Changes { removed_files: vec![file.id], ..Changes::default() }).await?;
            self.events
                .publish(FileDeleted { file: file.id, path: file.path.clone(), target: file.target, reason })
                .await;
            info!(path = %file.path.display(), "file deleted");

            for subtitle in subtitles {
                if let Err(error) = self.fs.remove_file(&subtitle.path).await {
                    warn!(%error, path = %subtitle.path.display(), "could not delete a subtitle of a deleted file");
                }
            }
            let root = roots.iter().map(|root| root.path.as_path()).find(|root| file.path.starts_with(root));
            if let (Some(root), Some(folder)) = (root, file.path.parent())
                && let Err(error) = self.fs.remove_empty_folders(folder, root).await
            {
                warn!(%error, folder = %folder.display(), "could not remove the folder of a deleted file");
            }
        }
        Ok(files)
    }

    /// Kept files leave the library but stay on disk.
    async fn remove_item(&self, item: ItemId, delete_files: bool) -> Result<(), MediaError> {
        let _lock = self.lock.acquire().await?;
        let files = self.files_of_item(item).await?;
        if files.is_empty() {
            return Ok(());
        }
        if delete_files {
            self.remove(files, DeleteReason::ItemRemoved).await?;
        } else {
            self.repo
                .save(&Changes { removed_files: files.iter().map(|file| file.id).collect(), ..Changes::default() })
                .await?;
            info!(kept = files.len(), "files of a removed item left the library");
        }
        Ok(())
    }
}

#[async_trait]
impl Handler<SeriesRemoved> for Deleter {
    async fn handle(&self, event: &SeriesRemoved) -> Result<(), HandlerError> {
        self.remove_item(ItemId::Series(event.series), event.delete_files).await?;
        Ok(())
    }
}

#[async_trait]
impl Handler<MovieRemoved> for Deleter {
    async fn handle(&self, event: &MovieRemoved) -> Result<(), HandlerError> {
        self.remove_item(ItemId::Movie(event.movie), event.delete_files).await?;
        Ok(())
    }
}
