use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use tempfile::TempDir;
use yokoku_detect::DownloadFile;
use yokoku_media::ports::FileSystem;
use yokoku_system::LocalFileSystem;

fn write(root: &Path, relative: &str, size: usize) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, vec![0; size]).unwrap();
    path
}

fn relative(root: &Path, files: &[DownloadFile]) -> Vec<(String, u64)> {
    files.iter().map(|file| (file.path.strip_prefix(root).unwrap().display().to_string(), file.size)).collect()
}

#[tokio::test]
async fn lists_files_at_any_depth_in_path_order() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "Frieren/Season 01/S01E02.mkv", 20);
    write(dir.path(), "Frieren/Season 01/S01E01.mkv", 10);
    write(dir.path(), "Dune.mkv", 5);

    let files = LocalFileSystem.files(dir.path()).await.unwrap();

    assert_eq!(
        relative(dir.path(), &files),
        [
            ("Dune.mkv".into(), 5),
            ("Frieren/Season 01/S01E01.mkv".into(), 10),
            ("Frieren/Season 01/S01E02.mkv".into(), 20),
        ]
    );
}

#[tokio::test]
async fn skips_hidden_entries_linked_folders_and_broken_links() {
    let dir = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let video = write(dir.path(), "Frieren/S01E01.mkv", 1);
    write(dir.path(), "Frieren/.S01E01.mkv.part", 1);
    write(dir.path(), ".trash/S01E02.mkv", 1);
    write(other.path(), "Pluto/S01E01.mkv", 1);
    symlink(other.path().join("Pluto"), dir.path().join("Pluto")).unwrap();
    symlink(&video, dir.path().join("Frieren/link.mkv")).unwrap();
    symlink(dir.path().join("missing.mkv"), dir.path().join("broken.mkv")).unwrap();

    let files = LocalFileSystem.files(dir.path()).await.unwrap();

    assert_eq!(relative(dir.path(), &files), [("Frieren/S01E01.mkv".into(), 1), ("Frieren/link.mkv".into(), 1)]);
}

#[tokio::test]
async fn a_missing_folder_is_an_error_not_an_empty_list() {
    let dir = TempDir::new().unwrap();

    let error = LocalFileSystem.files(&dir.path().join("unmounted")).await.unwrap_err();

    assert_eq!(error.path, dir.path().join("unmounted"));
}

#[tokio::test]
async fn an_unreadable_subfolder_fails_the_listing() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "Frieren/S01E01.mkv", 1);
    let locked = dir.path().join("Frieren");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let result = LocalFileSystem.files(dir.path()).await;

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(result.unwrap_err().path, locked);
}

#[tokio::test]
async fn tells_folders_from_files_and_missing_paths() {
    let dir = TempDir::new().unwrap();
    let file = write(dir.path(), "Dune.mkv", 1);

    assert!(LocalFileSystem.is_dir(dir.path()).await.unwrap());
    assert!(!LocalFileSystem.is_dir(&file).await.unwrap());
    assert!(!LocalFileSystem.is_dir(&dir.path().join("missing")).await.unwrap());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn skips_names_that_are_not_utf8() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let dir = TempDir::new().unwrap();
    write(dir.path(), "Dune.mkv", 1);
    fs::write(dir.path().join(OsStr::from_bytes(b"\xff.mkv")), b"").unwrap();

    let files = LocalFileSystem.files(dir.path()).await.unwrap();

    assert_eq!(relative(dir.path(), &files), [("Dune.mkv".into(), 1)]);
}
