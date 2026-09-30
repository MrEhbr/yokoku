use std::{
    io::{self, ErrorKind},
    path::PathBuf,
};

use async_trait::async_trait;
use tokio::fs;
use yokoku_core::library::ports::ArtworkCache;
use yokoku_domain::{ArtworkKind, ItemId, StorageError};

/// Artwork as files, one folder per item, each file named after its kind and image:
/// `series/<id>/poster-81189-10.jpg`.
#[derive(Debug, Clone)]
pub struct ArtworkFiles {
    folder: PathBuf,
}

impl ArtworkFiles {
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self { folder: folder.into() }
    }

    fn item_folder(&self, item: ItemId) -> PathBuf {
        match item {
            ItemId::Series(id) => self.folder.join("series").join(id.to_string()),
            ItemId::Movie(id) => self.folder.join("movie").join(id.to_string()),
        }
    }
}

#[async_trait]
impl ArtworkCache for ArtworkFiles {
    async fn get(&self, item: ItemId, kind: ArtworkKind, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        match fs::read(self.item_folder(item).join(format!("{kind}-{name}"))).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StorageError::new(error)),
        }
    }

    /// Writes a temporary file and renames it, so a reader never sees part of an image.
    async fn put(&self, item: ItemId, kind: ArtworkKind, name: &str, image: &[u8]) -> Result<(), StorageError> {
        let folder = self.item_folder(item);
        let file = format!("{kind}-{name}");
        async {
            fs::create_dir_all(&folder).await?;
            let partial = folder.join(format!(".{file}.partial"));
            fs::write(&partial, image).await?;
            fs::rename(&partial, folder.join(&file)).await?;
            let mut entries = fs::read_dir(&folder).await?;
            while let Some(entry) = entries.next_entry().await? {
                let other = entry.file_name();
                if other.to_str().is_some_and(|other| other.starts_with(&format!("{kind}-")) && other != file) {
                    fs::remove_file(entry.path()).await?;
                }
            }
            io::Result::Ok(())
        }
        .await
        .map_err(StorageError::new)
    }

    async fn remove(&self, item: ItemId) -> Result<(), StorageError> {
        match fs::remove_dir_all(self.item_folder(item)).await {
            Err(error) if error.kind() != ErrorKind::NotFound => Err(StorageError::new(error)),
            _ => Ok(()),
        }
    }
}
