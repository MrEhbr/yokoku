mod common;

use std::{fs, os::unix::fs::MetadataExt, path::Path};

use common::{App, relative};
use rstest::rstest;
use yokoku_domain::{DownloadId, ImportId, ItemId};
use yokoku_events::{DeleteReason, Event, FileDeleted, FilesImported, ImportFailed, LinkedFile};
use yokoku_library::ports::SeriesRepo;
use yokoku_media::{Approval, ImportMode, ImportStatus, MediaError, MediaFile, ports::MediaRepo};

const E01: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.mkv";
const SOURCE: &str = "downloads/Frieren.S01E01.1080p/Frieren.S01E01.1080p.mkv";

/// A finished download of Frieren S01E01 with an English subtitle, planned and approved.
async fn approved(app: &App) -> ImportId {
    app.write(SOURCE, 10);
    app.write("downloads/Frieren.S01E01.1080p/Frieren.S01E01.1080p.eng.srt", 2);
    let content = app.path("downloads/Frieren.S01E01.1080p");
    let import =
        app.planner.plan(DownloadId::generate(), &content, Some(ItemId::Series(app.frieren.id)), None).await.unwrap();
    let import = import.unwrap();
    assert_eq!(import.status, ImportStatus::Approved);
    import.id
}

fn inode(path: &Path) -> u64 {
    fs::metadata(path).unwrap().ino()
}

#[tokio::test]
async fn hard_links_the_video_and_its_subtitles_into_the_library() {
    let app = App::new().await;
    let id = approved(&app).await;

    let finished = app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert_eq!(
        finished.iter().map(|import| (import.id, import.status)).collect::<Vec<_>>(),
        [(id, ImportStatus::Done)]
    );
    assert_eq!(inode(&app.path(E01)), inode(&app.path(SOURCE)));
    assert!(app.path("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01 - Episode 1.en.srt").exists());
    let files = app.db_files().await;
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [E01]);
    assert_eq!(files[0].target, app.episodes(1, 1, 1));
    let linked = vec![LinkedFile { file: files[0].id, path: files[0].path.clone(), target: files[0].target }];
    let download = MediaRepo::import(&app.db, id).await.unwrap().unwrap().download;
    assert!(download.is_some());
    assert_eq!(app.events().await.last(), Some(&FilesImported { import: id, download, files: linked }.into()));
}

#[tokio::test]
async fn copies_leave_the_download_untouched() {
    let app = App::new().await;
    approved(&app).await;

    app.importer(ImportMode::Copy).run_pending().await.unwrap();

    assert_ne!(inode(&app.path(E01)), inode(&app.path(SOURCE)));
    assert_eq!(fs::read(app.path(E01)).unwrap().len(), 10);
}

#[tokio::test]
async fn moves_take_the_files_out_of_the_download() {
    let app = App::new().await;
    approved(&app).await;

    app.importer(ImportMode::Move).run_pending().await.unwrap();

    assert!(app.path(E01).exists());
    assert!(!app.path(SOURCE).exists());
}

#[tokio::test]
async fn an_import_interrupted_after_placing_files_completes_on_retry() {
    let app = App::new().await;
    let id = approved(&app).await;
    fs::create_dir_all(app.path(E01).parent().unwrap()).unwrap();
    fs::hard_link(app.path(SOURCE), app.path(E01)).unwrap();

    let finished = app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert_eq!(finished[0].status, ImportStatus::Done);
    assert_eq!(MediaRepo::import(&app.db, id).await.unwrap().unwrap().status, ImportStatus::Done);
}

#[tokio::test]
async fn a_blocked_destination_fails_the_import_until_retried() {
    let app = App::new().await;
    let id = approved(&app).await;
    let blocker = app.write(E01, 99);
    let importer = app.importer(ImportMode::HardLink);

    let failed = importer.run_pending().await.unwrap();
    fs::remove_file(&blocker).unwrap();
    importer.retry(id).await.unwrap();
    let retried = importer.run_pending().await.unwrap();

    assert_eq!(failed[0].status, ImportStatus::Failed);
    let reason = failed[0].error.clone().unwrap();
    assert!(reason.ends_with("already exists"), "{reason}");
    assert!(app.events().await.contains(&ImportFailed { import: id, source: failed[0].source.clone(), reason }.into()));
    assert_eq!(retried[0].status, ImportStatus::Done);
    assert!(matches!(importer.retry(id).await.unwrap_err(), MediaError::NotFailed(_)));
}

#[tokio::test]
async fn replacing_removes_the_old_library_file() {
    let app = App::new().await;
    let old = app.write("tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.720p.mkv", 5);
    app.scanner.scan().await.unwrap();
    let old_file = app.db_files().await.remove(0);
    app.write(SOURCE, 10);
    let import = app
        .planner
        .plan(DownloadId::generate(), &app.path(SOURCE), Some(ItemId::Series(app.frieren.id)), None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(import.status, ImportStatus::NeedsReview);
    app.reviewer.replace_row(import.id, 1).await.unwrap();
    assert_eq!(app.reviewer.approve(import.id).await.unwrap(), Approval::Queued);

    app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert!(!old.exists());
    let files = app.db_files().await;
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [E01]);
    let events = app.events().await;
    let deleted =
        FileDeleted { file: old_file.id, path: old, target: old_file.target, reason: DeleteReason::Replaced }.into();
    assert_eq!(events[events.len() - 2..].first(), Some(&deleted));
}

#[tokio::test]
async fn files_go_to_the_item_folder_after_its_title_changes() {
    let app = App::new().await;
    approved(&app).await;
    let mut frieren = app.frieren.clone();
    frieren.title = "Sousou no Frieren".into();
    SeriesRepo::save(&app.db, &mut frieren).await.unwrap();

    app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert!(app.path("tv/Frieren (2023)/Season 01/Sousou no Frieren (2023) - S01E01 - Episode 1.mkv").exists());
}

#[tokio::test]
async fn concurrent_runners_import_each_import_once() {
    let app = App::new().await;
    approved(&app).await;
    let (first, second) = (app.importer(ImportMode::HardLink), app.importer(ImportMode::HardLink));

    let (a, b) = tokio::join!(first.run_pending(), second.run_pending());

    assert_eq!(a.unwrap().len() + b.unwrap().len(), 1);
    let imported = app.events().await.iter().filter_map(Event::get::<FilesImported>).count();
    assert_eq!(imported, 1);
}

#[tokio::test]
async fn interrupted_imports_are_queued_again() {
    let app = App::new().await;
    let id = approved(&app).await;
    app.db.claim_next_approved().await.unwrap();
    let importer = app.importer(ImportMode::HardLink);

    assert_eq!(importer.list().await.unwrap()[0].status, ImportStatus::Importing);
    let finished = importer.run_pending().await.unwrap();

    assert_eq!(
        finished.iter().map(|import| (import.id, import.status)).collect::<Vec<_>>(),
        [(id, ImportStatus::Done)]
    );
}

const E02: &str = "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E02 - Episode 2.mkv";

/// A finished download of Frieren S01E01 and S01E02, planned and approved.
async fn approved_pair(app: &App) -> ImportId {
    app.write("downloads/Frieren.S01.1080p/Frieren.S01E01.1080p.mkv", 10);
    app.write("downloads/Frieren.S01.1080p/Frieren.S01E02.1080p.mkv", 10);
    let content = app.path("downloads/Frieren.S01.1080p");
    let import =
        app.planner.plan(DownloadId::generate(), &content, Some(ItemId::Series(app.frieren.id)), None).await.unwrap();
    let import = import.unwrap();
    assert_eq!(import.status, ImportStatus::Approved);
    import.id
}

#[tokio::test]
async fn a_failed_import_keeps_the_files_it_placed_and_completes_on_retry() {
    let app = App::new().await;
    let id = approved_pair(&app).await;
    let blocker = app.write(E02, 99);
    let importer = app.importer(ImportMode::HardLink);

    let failed = importer.run_pending().await.unwrap();
    let placed = app.db_files().await;
    fs::remove_file(&blocker).unwrap();
    importer.retry(id).await.unwrap();
    let retried = importer.run_pending().await.unwrap();

    assert_eq!(failed[0].status, ImportStatus::Failed);
    assert_eq!(relative(&app, placed.iter().map(|file| file.path.as_path())), [E01]);
    let imported: Vec<usize> = app
        .events()
        .await
        .iter()
        .filter_map(Event::get::<FilesImported>)
        .filter(|imported| imported.import == id)
        .map(|imported| imported.files.len())
        .collect();
    assert_eq!(imported, [1, 1]);
    assert_eq!(retried[0].status, ImportStatus::Done);
    let files = app.db_files().await;
    assert_eq!(relative(&app, files.iter().map(|file| file.path.as_path())), [E01, E02]);
    assert_eq!(files[0].id, placed[0].id);
}

/// A library file of 5 bytes at `E01` and a download of S01E01 kept beside it, approved.
async fn approved_beside(app: &App) -> ImportId {
    app.write(E01, 5);
    app.scanner.scan().await.unwrap();
    app.write(SOURCE, 10);
    let import = app
        .planner
        .plan(DownloadId::generate(), &app.path(SOURCE), Some(ItemId::Series(app.frieren.id)), None)
        .await
        .unwrap()
        .unwrap();
    app.reviewer.keep_both_row(import.id, 1).await.unwrap();
    assert_eq!(app.reviewer.approve(import.id).await.unwrap(), Approval::Queued);
    import.id
}

#[rstest]
#[case::next_number(&[], "Frieren (2023) - S01E01 - Episode 1 (2).mkv")]
#[case::past_a_taken_number(&["Frieren (2023) - S01E01 - Episode 1 (2).mkv"], "Frieren (2023) - S01E01 - Episode 1 (3).mkv")]
#[tokio::test]
async fn a_file_kept_beside_the_library_file_gets_the_next_free_name(#[case] taken: &[&str], #[case] name: &str) {
    let app = App::new().await;
    let id = approved_beside(&app).await;
    for other in taken {
        app.write(&format!("tv/Frieren (2023)/Season 01/{other}"), 3);
    }
    let rows = MediaRepo::import(&app.db, id).await.unwrap().unwrap().rows;
    let planned = app.importer(ImportMode::HardLink).destinations(&rows).await.unwrap();

    let finished = app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    let kept = format!("tv/Frieren (2023)/Season 01/{name}");
    assert_eq!(finished[0].status, ImportStatus::Done);
    assert_eq!(planned[0].as_ref().map(|destination| destination.path()), Some(app.path(&kept)));
    assert_eq!(fs::read(app.path(E01)).unwrap().len(), 5);
    assert_eq!(inode(&app.path(&kept)), inode(&app.path(SOURCE)));
    let mut files = relative(&app, app.db_files().await.iter().map(|file| file.path.as_path()));
    let mut expected = [E01.to_owned(), kept];
    files.sort();
    expected.sort();
    assert_eq!(files, expected);
    assert!(replaced(&app).await.is_empty());
}

/// A library file of `size` bytes at `old` replaced by an approved download of S01E01.
async fn approved_replacement(app: &App, old: &str, size: usize) -> MediaFile {
    app.write(old, size);
    app.scanner.scan().await.unwrap();
    let old_file = app.db_files().await.remove(0);
    app.write(SOURCE, 10);
    let import = app
        .planner
        .plan(DownloadId::generate(), &app.path(SOURCE), Some(ItemId::Series(app.frieren.id)), None)
        .await
        .unwrap()
        .unwrap();
    app.reviewer.replace_row(import.id, 1).await.unwrap();
    assert_eq!(app.reviewer.approve(import.id).await.unwrap(), Approval::Queued);
    old_file
}

fn folder_names(path: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

async fn replaced(app: &App) -> Vec<FileDeleted> {
    app.events().await.iter().filter_map(Event::get::<FileDeleted>).cloned().collect()
}

#[tokio::test]
async fn a_failed_replacement_keeps_the_old_file() {
    let app = App::new().await;
    let old_file = approved_replacement(&app, "tv/Frieren (2023)/Season 01/Frieren (2023) - S01E01.720p.mkv", 5).await;
    app.write(E01, 99);

    let failed = app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert_eq!(failed[0].status, ImportStatus::Failed);
    assert_eq!(fs::read(&old_file.path).unwrap().len(), 5);
    assert_eq!(app.db_files().await.iter().map(|file| file.id).collect::<Vec<_>>(), [old_file.id]);
    assert!(replaced(&app).await.is_empty());
}

#[tokio::test]
async fn replaces_a_file_at_the_same_path() {
    let app = App::new().await;
    let old_file = approved_replacement(&app, E01, 5).await;

    let finished = app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert_eq!(finished[0].status, ImportStatus::Done);
    assert_eq!(inode(&app.path(E01)), inode(&app.path(SOURCE)));
    assert_eq!(folder_names(&app.path(E01)), [E01.rsplit('/').next().unwrap()]);
    assert_eq!(replaced(&app).await.iter().map(|deleted| deleted.file).collect::<Vec<_>>(), [old_file.id]);
}

#[tokio::test]
async fn a_failed_replacement_at_the_same_path_restores_the_old_file() {
    let app = App::new().await;
    let old_file = approved_replacement(&app, E01, 5).await;
    fs::remove_file(app.path(SOURCE)).unwrap();

    let failed = app.importer(ImportMode::HardLink).run_pending().await.unwrap();

    assert_eq!(failed[0].status, ImportStatus::Failed);
    assert_eq!(fs::read(app.path(E01)).unwrap().len(), 5);
    assert_eq!(folder_names(&app.path(E01)), [E01.rsplit('/').next().unwrap()]);
    assert_eq!(app.db_files().await.iter().map(|file| file.id).collect::<Vec<_>>(), [old_file.id]);
    assert!(replaced(&app).await.is_empty());
}

#[tokio::test]
async fn a_different_file_of_the_same_size_blocks_the_destination() {
    let app = App::new().await;
    approved(&app).await;
    let blocker = app.path(E01);
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, [1; 10]).unwrap();

    let finished = app.importer(ImportMode::Copy).run_pending().await.unwrap();

    assert_eq!(finished[0].status, ImportStatus::Failed);
    assert_eq!(fs::read(&blocker).unwrap(), [1; 10]);
}

#[tokio::test]
async fn an_interrupted_copy_completes_on_retry() {
    let app = App::new().await;
    approved(&app).await;
    fs::create_dir_all(app.path(E01).parent().unwrap()).unwrap();
    fs::copy(app.path(SOURCE), app.path(E01)).unwrap();

    let finished = app.importer(ImportMode::Copy).run_pending().await.unwrap();

    assert_eq!(finished[0].status, ImportStatus::Done);
}

#[tokio::test]
async fn running_and_retrying_imports_is_announced() {
    let app = App::new().await;
    let id = approved(&app).await;
    fs::create_dir_all(app.path(E01)).unwrap();
    let mut watch = app.changes.watch();
    let importer = app.importer(ImportMode::HardLink);

    importer.run_pending().await.unwrap();
    let after_run = watch.has_changed().unwrap();
    watch.borrow_and_update();
    importer.retry(id).await.unwrap();

    assert!(after_run);
    assert!(watch.has_changed().unwrap());
}
