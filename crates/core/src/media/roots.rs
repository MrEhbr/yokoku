use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::media::{
    MediaError, RootFolder, RootKind,
    ports::{Catalog, FileSystem, MediaRepo},
};

/// Root folder settings.
pub struct RootFolders {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
}

impl RootFolders {
    pub fn new(repo: Arc<dyn MediaRepo>, catalog: Arc<dyn Catalog>, fs: Arc<dyn FileSystem>) -> Self {
        Self { repo, catalog, fs }
    }

    pub async fn list(&self) -> Result<Vec<RootFolder>, MediaError> {
        Ok(self.repo.root_folders().await?)
    }

    /// The root folder of `kind` at `path`, for adding an item to it.
    pub async fn get(&self, kind: RootKind, path: &Path) -> Result<RootFolder, MediaError> {
        let path: PathBuf = path.components().collect();
        let root = self.list().await?.into_iter().find(|root| root.path == path);
        match root {
            Some(root) if root.kind == kind => Ok(root),
            Some(root) => Err(MediaError::WrongRootKind { path, kind: root.kind }),
            None => Err(MediaError::RootNotFound(path)),
        }
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

    /// Names of the folders in `root`, such as ones an added item can take over.
    pub async fn folders(&self, root: &RootFolder) -> Result<Vec<String>, MediaError> {
        Ok(self.fs.folders(&root.path).await?)
    }

    /// Names of the folders of the series and movies in `root`, whether they exist yet or not.
    pub async fn item_folders(&self, root: &RootFolder) -> Result<Vec<String>, MediaError> {
        let series = self.catalog.all_series().await?.into_iter().map(|series| series.folder);
        let movies = self.catalog.all_movies().await?.into_iter().map(|movie| movie.folder);
        Ok(series.chain(movies).filter(|folder| folder.root == root.path).map(|folder| folder.name).collect())
    }

    /// Refused while series or movies belong to the folder.
    pub async fn remove(&self, path: &Path) -> Result<(), MediaError> {
        let path: PathBuf = path.components().collect();
        let series = self.catalog.all_series().await?;
        let movies = self.catalog.all_movies().await?;
        let items = series.iter().filter(|series| series.folder.root == path).count()
            + movies.iter().filter(|movie| movie.folder.root == path).count();
        if items > 0 {
            return Err(MediaError::RootInUse { path, items });
        }
        if !self.repo.remove_root_folder(&path).await? {
            return Err(MediaError::RootNotFound(path));
        }
        Ok(())
    }
}
