use std::{
    fs, io,
    os::unix::fs::MetadataExt,
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
        blocking(move || walk(&dir, true)).await
    }

    async fn files_in(&self, dir: &Path) -> Result<Vec<DownloadFile>, FsError> {
        let dir = dir.to_owned();
        blocking(move || walk(&dir, false)).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let (from, to) = (from.to_owned(), to.to_owned());
        blocking(move || rename(&from, &to)).await
    }

    async fn remove_empty_folders(&self, dir: &Path, stop: &Path) -> Result<(), FsError> {
        let (dir, stop) = (dir.to_owned(), stop.to_owned());
        blocking(move || remove_empty_folders(&dir, &stop)).await
    }
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, FsError> + Send + 'static) -> Result<T, FsError> {
    task::spawn_blocking(work)
        .await
        .map_err(|error| FsError { path: PathBuf::new(), source: io::Error::other(error) })?
}

fn at(path: &Path) -> impl FnOnce(io::Error) -> FsError + '_ {
    move |source| FsError { path: path.to_owned(), source }
}

/// Any unreadable folder fails the whole walk, so a missing folder never looks empty.
fn walk(root: &Path, recursive: bool) -> Result<Vec<DownloadFile>, FsError> {
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
                if recursive {
                    folders.push(path);
                }
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

/// Creates missing folders; never replaces another file. A change of letter case alone is allowed
/// on case-insensitive file systems.
fn rename(from: &Path, to: &Path) -> Result<(), FsError> {
    match (fs::metadata(from), fs::metadata(to)) {
        (Ok(source), Ok(target)) if source.dev() != target.dev() || source.ino() != target.ino() => {
            return Err(FsError { path: to.to_owned(), source: io::ErrorKind::AlreadyExists.into() });
        },
        (Err(source), _) => return Err(FsError { path: from.to_owned(), source }),
        _ => {},
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(at(parent))?;
    }
    fs::rename(from, to).map_err(at(from))
}

/// Removes `dir` and then each parent while they are empty, stopping before `stop`.
fn remove_empty_folders(dir: &Path, stop: &Path) -> Result<(), FsError> {
    for folder in dir.ancestors().take_while(|folder| *folder != stop && folder.starts_with(stop)) {
        match fs::remove_dir(folder) {
            Ok(()) => {},
            Err(error) if matches!(error.kind(), io::ErrorKind::DirectoryNotEmpty | io::ErrorKind::NotFound) => {
                return Ok(());
            },
            Err(source) => return Err(FsError { path: folder.to_owned(), source }),
        }
    }
    Ok(())
}
