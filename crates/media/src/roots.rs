use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    MediaError, RootFolder, RootKind,
    ports::{FileSystem, MediaRepo},
};

/// Root folder settings (FR-8.1).
pub struct RootFolders {
    repo: Arc<dyn MediaRepo>,
    fs: Arc<dyn FileSystem>,
}

impl RootFolders {
    pub fn new(repo: Arc<dyn MediaRepo>, fs: Arc<dyn FileSystem>) -> Self {
        Self { repo, fs }
    }

    pub async fn list(&self) -> Result<Vec<RootFolder>, MediaError> {
        Ok(self.repo.root_folders().await?)
    }

    /// The path must be an existing absolute folder that neither contains nor lies inside
    /// another root folder.
    pub async fn add(&self, kind: RootKind, path: &Path) -> Result<RootFolder, MediaError> {
        if !path.is_absolute() {
            return Err(MediaError::RelativePath(path.to_owned()));
        }
        let path: PathBuf = path.components().collect();
        if !self.fs.is_dir(&path).await? {
            return Err(MediaError::NotAFolder(path));
        }
        let roots = self.repo.root_folders().await?;
        if let Some(existing) =
            roots.into_iter().find(|root| path.starts_with(&root.path) || root.path.starts_with(&path))
        {
            return Err(MediaError::OverlappingRoot { path, existing: existing.path });
        }

        let root = RootFolder { kind, path };
        self.repo.add_root_folder(&root).await?;
        Ok(root)
    }

    /// Files under the folder stay linked to the library.
    pub async fn remove(&self, path: &Path) -> Result<(), MediaError> {
        let path: PathBuf = path.components().collect();
        if !self.repo.remove_root_folder(&path).await? {
            return Err(MediaError::RootNotFound(path));
        }
        Ok(())
    }
}
