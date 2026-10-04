use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tracing::warn;
use yokoku_domain::{DiskSpace, ItemId};

use crate::media::{
    MediaError, RootFolder, RootKind,
    ports::{Catalog, FileSystem, FsError, MediaRepo},
};

/// Root folder settings: the ones in the config file and the stored ones.
pub struct RootFolders {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    configured: Vec<RootFolder>,
}

impl RootFolders {
    /// `configured` are the root folders of the config file.
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        configured: Vec<RootFolder>,
    ) -> Self {
        let configured = configured
            .into_iter()
            .map(|root| RootFolder { path: root.path.components().collect(), configured: true, ..root })
            .collect();
        Self { repo, catalog, fs, configured }
    }

    /// The configured root folders and the stored ones, by path; a stored one at the path of a
    /// configured one is left out.
    pub async fn list(&self) -> Result<Vec<RootFolder>, MediaError> {
        let stored = self.repo.root_folders().await?;
        let mut roots = self.configured.clone();
        roots.extend(stored.into_iter().filter(|root| !self.configured.iter().any(|known| known.path == root.path)));
        roots.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(roots)
    }

    /// Fails when a configured root folder is relative, overlaps another configured one, or
    /// conflicts with a stored one: the same path with another kind, or one inside the other.
    pub async fn check(&self) -> Result<(), MediaError> {
        for (index, root) in self.configured.iter().enumerate() {
            if !root.path.is_absolute() {
                return Err(MediaError::RelativePath(root.path.clone()));
            }
            if let Some(other) = self.configured[index + 1..].iter().find(|other| overlap(&root.path, &other.path)) {
                return Err(MediaError::OverlappingRoot { path: other.path.clone(), existing: root.path.clone() });
            }
        }
        for stored in self.repo.root_folders().await? {
            let conflict = self.configured.iter().find(|root| {
                if root.path == stored.path { root.kind != stored.kind } else { overlap(&root.path, &stored.path) }
            });
            if let Some(root) = conflict {
                return Err(MediaError::ConflictingRoot { configured: root.path.clone(), stored: stored.path });
            }
        }
        Ok(())
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
    pub async fn add(&self, kind: RootKind, path: &Path, name: Option<String>) -> Result<RootFolder, MediaError> {
        if !path.is_absolute() {
            return Err(MediaError::RelativePath(path.to_owned()));
        }
        let path: PathBuf = path.components().collect();
        if !self.fs.is_dir(&path).await? {
            return Err(MediaError::NotAFolder(path));
        }
        if let Some(existing) = self.list().await?.into_iter().find(|root| overlap(&path, &root.path)) {
            return Err(MediaError::OverlappingRoot { path, existing: existing.path });
        }

        let root = RootFolder::new(kind, path, name, false);
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

    /// The listed root folder `item` lives in.
    pub async fn of(&self, item: ItemId) -> Result<Option<RootFolder>, MediaError> {
        let root = match item {
            ItemId::Series(id) => self.catalog.series(id).await?.map(|series| series.folder.root),
            ItemId::Movie(id) => self.catalog.movie(id).await?.map(|movie| movie.folder.root),
        };
        let Some(root) = root else { return Ok(None) };
        Ok(self.list().await?.into_iter().find(|listed| listed.path == root))
    }

    /// The space left on the file system holding `root`.
    pub async fn space(&self, root: &RootFolder) -> Result<DiskSpace, MediaError> {
        Ok(self.fs.space(&root.path).await?)
    }

    /// The root folders grouped by the file system holding them, with its space; ones that cannot
    /// be read are left out.
    pub async fn spaces(&self) -> Result<Vec<(Vec<RootFolder>, DiskSpace)>, MediaError> {
        let mut disks: Vec<(u64, Vec<RootFolder>, DiskSpace)> = Vec::new();
        for root in self.list().await? {
            match self.disk(&root).await {
                Ok(Some((device, space))) => match disks.iter_mut().find(|(known, ..)| *known == device) {
                    Some((_, roots, _)) => roots.push(root),
                    None => disks.push((device, vec![root], space)),
                },
                Ok(None) => {},
                Err(error) => warn!(%error, "reading a root folder's space failed"),
            }
        }
        Ok(disks.into_iter().map(|(_, roots, space)| (roots, space)).collect())
    }

    /// The device holding `root` and its space; `None` when the folder is gone.
    async fn disk(&self, root: &RootFolder) -> Result<Option<(u64, DiskSpace)>, FsError> {
        let Some(stat) = self.fs.stat(&root.path).await? else { return Ok(None) };
        Ok(Some((stat.device, self.fs.space(&root.path).await?)))
    }

    /// Refused for a configured root folder, and while series or movies belong to the folder.
    pub async fn remove(&self, path: &Path) -> Result<(), MediaError> {
        let path: PathBuf = path.components().collect();
        if self.configured.iter().any(|root| root.path == path) {
            return Err(MediaError::ConfiguredRoot(path));
        }
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

/// One path is the other or lies inside it.
fn overlap(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
