mod common;

use std::time::Duration;

use common::App;
use rstest::rstest;
use tokio::time::timeout;
use yokoku_events::{Handler, SeriesRemoved};
use yokoku_media::{ImportMode, RenameScope, ports::LibraryLock};

#[derive(Debug, Clone, Copy)]
enum Change {
    Scan,
    Import,
    Rename,
    Delete,
    RemoveSeries,
}

async fn run(app: &App, change: Change) {
    match change {
        Change::Scan => drop(app.scanner.scan().await),
        Change::Import => drop(app.importer(ImportMode::HardLink).run_pending().await),
        Change::Rename => drop(app.renamer.apply(RenameScope::All, None).await),
        Change::Delete => drop(app.deleter().delete(app.movie()).await),
        Change::RemoveSeries => {
            let removed = SeriesRemoved { series: app.frieren.id, title: "Frieren".into(), delete_files: true };
            drop(app.deleter().handle(&removed).await);
        },
    }
}

#[rstest]
#[tokio::test]
async fn library_changes_wait_for_the_lock(
    #[values(Change::Scan, Change::Import, Change::Rename, Change::Delete, Change::RemoveSeries)] change: Change,
) {
    let app = App::new().await;
    let held = app.lock().acquire().await.unwrap();

    let waiting = timeout(Duration::from_millis(100), run(&app, change)).await;
    assert!(waiting.is_err(), "{change:?} ran while the lock was held");

    drop(held);
    timeout(Duration::from_secs(5), run(&app, change)).await.unwrap();
}
