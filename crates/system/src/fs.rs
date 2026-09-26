use std::{
    fs, io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tokio::task;
use tracing::warn;
use yokoku_detect::DownloadFile;
use yokoku_media::ports::{FileSystem, FsError};

#[derive(Debug, Clone, Default)]
pub struct LocalFileSystem;

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn is_dir(&self, path: &Path) -> Result<bool, FsError> {
        match tokio::fs::metadata(path).await {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(FsError { path: path.to_owned(), source }),
        }
    }

    async fn files(&self, dir: &Path) -> Result<Vec<DownloadFile>, FsError> {
        let dir = dir.to_owned();
        task::spawn_blocking(move || walk(&dir))
            .await
            .map_err(|error| FsError { path: PathBuf::new(), source: io::Error::other(error) })?
    }
}

fn at(path: &Path) -> impl FnOnce(io::Error) -> FsError + '_ {
    move |source| FsError { path: path.to_owned(), source }
}

/// Any unreadable folder fails the whole walk, so a missing folder never looks empty.
fn walk(root: &Path) -> Result<Vec<DownloadFile>, FsError> {
    let mut files = Vec::new();
    let mut folders = vec![root.to_owned()];

    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).map_err(at(&folder))? {
            let entry = entry.map_err(at(&folder))?;
            let path = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                warn!(path = %path.display(), "skipping a name that is not UTF-8");
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            if entry.file_type().map_err(at(&path))?.is_dir() {
                folders.push(path);
                continue;
            }
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() => files.push(DownloadFile { path, size: metadata.len() }),
                Ok(_) => {},
                Err(source) if source.kind() == io::ErrorKind::NotFound => {
                    warn!(path = %path.display(), "skipping a broken link");
                },
                Err(source) => return Err(FsError { path, source }),
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
