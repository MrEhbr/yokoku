use std::{fs::OpenOptions, path::PathBuf};

use async_trait::async_trait;
use yokoku_core::media::ports::{FsError, LibraryLock, LockGuard};

use crate::system::fs::blocking;

/// An exclusive `flock` on a file; every open of the same path contends, in this process or another.
#[derive(Debug, Clone)]
pub struct LockFile {
    path: PathBuf,
}

impl LockFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl LibraryLock for LockFile {
    async fn acquire(&self) -> Result<LockGuard, FsError> {
        let path = self.path.clone();
        blocking(move || {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&path)
                .map_err(|source| FsError::new(&path, source))?;
            file.lock().map_err(|source| FsError::new(&path, source))?;
            Ok(LockGuard::new(file))
        })
        .await
    }
}
