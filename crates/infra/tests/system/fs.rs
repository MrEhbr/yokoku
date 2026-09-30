use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use tempfile::TempDir;
use yokoku_core::media::{detect::ListedFile, ports::FileSystem};
use yokoku_infra::system::LocalFileSystem;

fn write(root: &Path, relative: &str, size: usize) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, vec![0; size]).unwrap();
    path
}

fn relative(root: &Path, files: &[ListedFile]) -> Vec<(String, u64)> {
    files.iter().map(|file| (file.path.strip_prefix(root).unwrap().display().to_string(), file.size)).collect()
}

#[tokio::test]
async fn lists_the_visible_folders_directly_in_a_folder_by_name() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "Frieren/Season 01/S01E01.mkv", 1);
    write(dir.path(), "Arcane (2021)/S01E01.mkv", 1);
    write(dir.path(), ".cache/x", 1);
    write(dir.path(), "Dune.mkv", 1);

    let folders = LocalFileSystem.folders(dir.path()).await.unwrap();
    let missing = LocalFileSystem.folders(&dir.path().join("gone")).await.unwrap();

    assert_eq!(folders, ["Arcane (2021)", "Frieren"]);
    assert!(missing.is_empty());
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
async fn lists_a_hidden_folder_itself() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join(".downloads");
    write(&root, "Dune.mkv", 1);

    let files = LocalFileSystem.files(&root).await.unwrap();

    assert_eq!(relative(&root, &files), [("Dune.mkv".into(), 1)]);
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

#[tokio::test]
async fn lists_only_the_files_directly_in_a_folder() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "Season 01/S01E01.mkv", 1);
    write(dir.path(), "Season 01/S01E01.en.srt", 2);
    write(dir.path(), "Season 01/Subs/S01E01.ru.srt", 3);

    let files = LocalFileSystem.files_in(&dir.path().join("Season 01")).await.unwrap();

    assert_eq!(
        relative(dir.path(), &files),
        [("Season 01/S01E01.en.srt".into(), 2), ("Season 01/S01E01.mkv".into(), 1)]
    );
}

#[tokio::test]
async fn renames_into_new_folders() {
    let dir = TempDir::new().unwrap();
    let from = write(dir.path(), "a.mkv", 3);
    let to = dir.path().join("Frieren (2023)/Season 01/b.mkv");

    LocalFileSystem.rename(&from, &to).await.unwrap();

    assert!(!from.exists());
    assert_eq!(fs::read(&to).unwrap().len(), 3);
}

#[tokio::test]
async fn never_renames_over_another_file() {
    let dir = TempDir::new().unwrap();
    let from = write(dir.path(), "a.mkv", 3);
    let to = write(dir.path(), "b.mkv", 5);

    let error = LocalFileSystem.rename(&from, &to).await.unwrap_err();

    assert_eq!(error.source.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(&to).unwrap().len(), 5);
    assert!(from.exists());
}

#[tokio::test]
async fn renaming_a_missing_file_fails() {
    let dir = TempDir::new().unwrap();

    let error = LocalFileSystem.rename(&dir.path().join("a.mkv"), &dir.path().join("b.mkv")).await.unwrap_err();

    assert_eq!(error.path, dir.path().join("a.mkv"));
}

#[tokio::test]
async fn removes_empty_folders_up_to_the_stop() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "tv/Kept/S01E01.mkv", 1);
    fs::create_dir_all(dir.path().join("tv/Kept/Old/Season 01")).unwrap();
    fs::create_dir_all(dir.path().join("tv/Gone/Season 01")).unwrap();
    let tv = dir.path().join("tv");

    LocalFileSystem.remove_empty_folders(&tv.join("Gone/Season 01"), &tv).await.unwrap();
    LocalFileSystem.remove_empty_folders(&tv.join("Kept/Old/Season 01"), &tv).await.unwrap();

    assert!(!tv.join("Gone").exists());
    assert!(!tv.join("Kept/Old").exists());
    assert!(tv.join("Kept/S01E01.mkv").exists());
    assert!(tv.exists());
}

#[tokio::test]
async fn changes_only_the_letter_case() {
    let dir = TempDir::new().unwrap();
    let from = write(dir.path(), "frieren.mkv", 3);

    LocalFileSystem.rename(&from, &dir.path().join("Frieren.mkv")).await.unwrap();

    let names: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, ["Frieren.mkv"]);
}

#[tokio::test]
async fn stat_tells_sizes_and_linked_files_apart() {
    let dir = TempDir::new().unwrap();
    let original = write(dir.path(), "Dune.mkv", 7);
    let copy = write(dir.path(), "copy.mkv", 7);
    let link = dir.path().join("Movies/Dune (2021)/Dune (2021).mkv");

    LocalFileSystem.hard_link(&original, &link).await.unwrap();

    let stat = |path: PathBuf| async move { LocalFileSystem.stat(&path).await.unwrap().unwrap() };
    let (original, copy, link) = (stat(original).await, stat(copy).await, stat(link).await);
    assert_eq!((original.size, link.size), (7, 7));
    assert!(original.same_file(&link));
    assert!(!original.same_file(&copy));
    assert_eq!(LocalFileSystem.stat(&dir.path().join("missing")).await.unwrap(), None);
}

#[tokio::test]
async fn compares_contents_byte_for_byte() {
    let dir = TempDir::new().unwrap();
    let original = write(dir.path(), "Dune.mkv", 100_000);
    let copy = write(dir.path(), "copy.mkv", 100_000);
    let different = write(dir.path(), "different.mkv", 100_000);
    let mut bytes = fs::read(&different).unwrap();
    bytes[99_999] = 1;
    fs::write(&different, bytes).unwrap();
    let shorter = write(dir.path(), "shorter.mkv", 99_999);

    assert!(LocalFileSystem.same_contents(&original, &copy).await.unwrap());
    assert!(!LocalFileSystem.same_contents(&original, &different).await.unwrap());
    assert!(!LocalFileSystem.same_contents(&original, &shorter).await.unwrap());
    assert!(!LocalFileSystem.same_contents(&shorter, &original).await.unwrap());
}

#[tokio::test]
async fn copies_into_new_folders_without_leaving_partial_files() {
    let dir = TempDir::new().unwrap();
    let from = write(dir.path(), "downloads/Dune.mkv", 9);
    let to = dir.path().join("Movies/Dune (2021)/Dune (2021).mkv");

    LocalFileSystem.copy(&from, &to).await.unwrap();

    assert_eq!(fs::read(&to).unwrap().len(), 9);
    assert!(from.exists());
    let names: Vec<_> = fs::read_dir(to.parent().unwrap()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names, ["Dune (2021).mkv"]);
}

#[tokio::test]
async fn links_and_copies_never_replace_a_file() {
    let dir = TempDir::new().unwrap();
    let from = write(dir.path(), "a.mkv", 3);
    let to = write(dir.path(), "b.mkv", 5);

    let link = LocalFileSystem.hard_link(&from, &to).await.unwrap_err();
    let copy = LocalFileSystem.copy(&from, &to).await.unwrap_err();

    assert_eq!(link.source.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(copy.source.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(&to).unwrap().len(), 5);
}

#[tokio::test]
async fn removing_a_missing_file_is_fine() {
    let dir = TempDir::new().unwrap();
    let file = write(dir.path(), "a.mkv", 3);

    LocalFileSystem.remove_file(&file).await.unwrap();
    LocalFileSystem.remove_file(&file).await.unwrap();

    assert!(!file.exists());
}
