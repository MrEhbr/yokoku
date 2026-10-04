#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use jiff::{Timestamp, civil::date};
use tempfile::TempDir;
use yokoku_core::{
    events::{EventLog, Publisher, QueueChanges},
    library::ports::{MovieRepo, SeriesRepo},
    media::{
        ImportMode, ImportPlanner, ImportSettings, Importer, Renamer, Reviewer, RootFolder, RootFolders, RootKind,
        Scanner,
        ports::{MergeError, Merger, Track},
    },
};
use yokoku_domain::{
    Clock, EpisodeSpan, FileTarget, ItemFolder, Live, MonitorPreset, Movie, MovieMetadata, Releases, Series,
    SeriesMetadata, SourceStatus, events::Event, naming::Naming,
};
use yokoku_infra::{
    db::Database,
    system::{LocalFileSystem, LockFile},
};
pub use yokoku_test_support::clock::TODAY;
use yokoku_test_support::{
    clock::TestClock,
    events::publisher,
    metadata::{movie_metadata, series_metadata},
};

/// The library lock file, in the test folder.
pub const LOCK: &str = "library.lock";

pub fn now() -> Timestamp {
    TestClock::default().now().timestamp()
}

/// Series and movie root folders on disk, "Frieren (2023)" with two seasons of three episodes in
/// `tv/Frieren (2023)`, and "Dune (2021)" in `movies/Dune (2021)`.
pub struct App {
    pub dir: TempDir,
    pub db: Database,
    pub roots: Arc<RootFolders>,
    pub scanner: Scanner,
    pub reviewer: Reviewer,
    pub renamer: Renamer,
    pub planner: ImportPlanner,
    /// Shared by every use case the app builds.
    pub changes: QueueChanges,
    pub frieren: Series,
    pub dune: Movie,
}

impl App {
    pub async fn new() -> Self {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("tv")).unwrap();
        fs::create_dir_all(dir.path().join("movies")).unwrap();

        let db = Database::open_in_memory().await.unwrap();
        let repo = Arc::new(db.clone());
        let events = publisher(&db);
        let fs = Arc::new(LocalFileSystem);
        let clock = Arc::new(TestClock::default());
        let roots = Arc::new(RootFolders::new(repo.clone(), repo.clone(), fs.clone(), Vec::new()));
        let lock = Arc::new(LockFile::new(dir.path().join(LOCK)));
        let changes = QueueChanges::new();
        let scanner =
            Scanner::new(repo.clone(), repo.clone(), fs, lock.clone(), clock.clone(), events.clone(), changes.clone());
        let reviewer = Reviewer::new(repo.clone(), repo.clone(), clock, events.clone(), changes.clone());
        let renamer = Renamer::new(
            repo.clone(),
            repo.clone(),
            roots.clone(),
            Arc::new(LocalFileSystem),
            lock,
            Live::fixed(Naming::default()),
            events.clone(),
        );
        let planner = ImportPlanner::new(
            repo.clone(),
            repo,
            Arc::new(LocalFileSystem),
            Arc::new(TestClock::default()),
            events.clone(),
            changes.clone(),
        );

        let tv = ItemFolder::new(dir.path().join("tv"), "Frieren (2023)".into()).unwrap();
        let movies = ItemFolder::new(dir.path().join("movies"), "Dune (2021)".into()).unwrap();
        let mut frieren = Series::new(frieren_metadata(), tv, MonitorPreset::All, TODAY, now());
        let mut dune = Movie::new(dune_metadata(), movies, true, now());
        SeriesRepo::save(&db, &mut frieren).await.unwrap();
        MovieRepo::save(&db, &mut dune).await.unwrap();

        let app = Self { dir, db, roots, scanner, reviewer, renamer, planner, changes, frieren, dune };
        app.roots.add(RootKind::Series, &app.path("tv"), None).await.unwrap();
        app.roots.add(RootKind::Movies, &app.path("movies"), None).await.unwrap();
        app
    }

    /// Root folders with `configured` from the config file, over the stored `tv` and `movies`.
    pub fn roots_with(&self, configured: Vec<RootFolder>) -> RootFolders {
        let repo = Arc::new(self.db.clone());
        RootFolders::new(repo.clone(), repo, Arc::new(LocalFileSystem), configured)
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// Creates a file of `size` bytes, with any missing folders.
    pub fn write(&self, relative: &str, size: usize) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![0; size]).unwrap();
        path
    }

    pub fn episodes(&self, season: u16, first: u16, last: u16) -> FileTarget {
        FileTarget::Episodes { series: self.frieren.id, span: EpisodeSpan::new(season, first, last).unwrap() }
    }

    pub fn movie(&self) -> FileTarget {
        FileTarget::Movie(self.dune.id)
    }

    pub async fn events(&self) -> Vec<Event> {
        let recorded = EventLog::new(self.db.clone()).read_after(None, 100).await.unwrap();
        recorded.into_iter().map(|recorded| recorded.event).collect()
    }

    pub async fn db_files(&self) -> Vec<yokoku_core::media::MediaFile> {
        yokoku_core::media::ports::MediaRepo::files(&self.db).await.unwrap()
    }

    /// Writes each of `paths` and scans them into the library; the stored files after.
    pub async fn linked(&self, paths: &[&str]) -> Vec<yokoku_core::media::MediaFile> {
        for path in paths {
            self.write(path, 10);
        }
        self.scanner.scan().await.unwrap();
        self.db_files().await
    }

    /// An importer that places external tracks beside their video.
    pub fn importer(&self, mode: ImportMode) -> Importer {
        self.merging_importer(mode, false, Arc::new(FakeMerger::default()))
    }

    /// An importer that merges external tracks into their video with `merger` when `merge` is set.
    pub fn merging_importer(&self, mode: ImportMode, merge: bool, merger: Arc<FakeMerger>) -> Importer {
        let repo = Arc::new(self.db.clone());
        Importer::new(
            repo.clone(),
            repo,
            Arc::new(LocalFileSystem),
            self.lock(),
            Arc::new(TestClock::default()),
            Live::fixed(Naming::default()),
            Live::fixed(ImportSettings { mode, merge }),
            merger,
            self.publisher(),
            self.changes.clone(),
        )
    }

    pub fn deleter(&self) -> yokoku_core::media::Deleter {
        yokoku_core::media::Deleter::new(
            Arc::new(self.db.clone()),
            self.roots.clone(),
            Arc::new(LocalFileSystem),
            self.lock(),
            self.publisher(),
        )
    }

    pub fn publisher(&self) -> Publisher {
        publisher(&self.db)
    }

    pub fn lock(&self) -> Arc<LockFile> {
        Arc::new(LockFile::new(self.path(LOCK)))
    }
}

/// Two seasons of three episodes, all aired on 2023-09-29.
pub fn frieren_metadata() -> SeriesMetadata {
    let aired = [Some(date(2023, 9, 29)); 3];
    SeriesMetadata {
        original_title: "Sousou no Frieren".into(),
        ..series_metadata(209867, "Frieren", SourceStatus::Returning, &[(1, &aired), (2, &aired)])
    }
}

fn dune_metadata() -> MovieMetadata {
    MovieMetadata { year: Some(2021), ..movie_metadata(438631, "Dune", Releases::default()) }
}

pub fn relative<'a>(app: &App, paths: impl IntoIterator<Item = &'a Path>) -> Vec<String> {
    paths.into_iter().map(|path| path.strip_prefix(app.dir.path()).unwrap().display().to_string()).collect()
}

/// Writes the video and the tracks, concatenated, to where it merges, and records the tracks; with
/// `fails`, it writes part of the file and fails instead.
#[derive(Default)]
pub struct FakeMerger {
    pub fails: bool,
    pub merged: std::sync::Mutex<Vec<Vec<Track>>>,
}

#[async_trait::async_trait]
impl Merger for FakeMerger {
    async fn merge(&self, video: &Path, tracks: &[Track], to: &Path) -> Result<(), MergeError> {
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        let mut bytes = fs::read(video).unwrap();
        if self.fails {
            fs::write(to, &bytes[..bytes.len() / 2]).unwrap();
            return Err(MergeError::Failed { path: to.to_owned(), reason: "broken track".into() });
        }
        for track in tracks {
            bytes.extend(fs::read(&track.path).unwrap());
        }
        fs::write(to, bytes).unwrap();
        self.merged.lock().unwrap().push(tracks.to_vec());
        Ok(())
    }
}
