use std::time::Duration;

use tempfile::TempDir;
use tokio::time::timeout;
use yokoku_media::ports::LibraryLock;
use yokoku_system::LockFile;

#[tokio::test]
async fn a_second_holder_waits_until_the_first_releases() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("library.lock");
    let first = LockFile::new(&path).acquire().await.unwrap();

    let second = tokio::spawn(async move { LockFile::new(path).acquire().await.map(drop) });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!second.is_finished());

    drop(first);
    timeout(Duration::from_secs(5), second).await.unwrap().unwrap().unwrap();
}

#[tokio::test]
async fn fails_when_the_folder_is_missing() {
    let dir = TempDir::new().unwrap();
    let error = LockFile::new(dir.path().join("missing/library.lock")).acquire().await.err().unwrap();

    assert_eq!(error.path, dir.path().join("missing/library.lock"));
}
