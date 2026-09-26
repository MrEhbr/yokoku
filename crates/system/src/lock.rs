use std::{fs::OpenOptions, path::PathBuf};

use async_trait::async_trait;
use yokoku_media::ports::{FsError, LibraryLock, LockGuard};

use crate::fs::{at, blocking};

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
            let file = OpenOptions::new().create(true).write(true).truncate(false).open(&path).map_err(at(&path))?;
            file.lock().map_err(at(&path))?;
            Ok(LockGuard::new(file))
        })
        .await
    }
}
