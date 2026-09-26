#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use jiff::{
    Timestamp, Zoned,
    civil::{Date, date},
    tz::TimeZone,
};
use tempfile::TempDir;
use yokoku_db::Database;
use yokoku_domain::{
    Clock, EpisodeMetadata, EpisodeRef, EpisodeSpan, ExternalId, FileTarget, MonitorPreset, Movie, MovieMetadata,
    Releases, SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};
use yokoku_events::{Event, EventLog};
use yokoku_library::ports::{MovieRepo, SeriesRepo};
use yokoku_media::{ImportPlanner, Renamer, Review, RootFolders, RootKind, Scanner};
use yokoku_naming::Naming;
use yokoku_system::LocalFileSystem;

pub const TODAY: Date = date(2026, 9, 26);

pub struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Zoned {
        TODAY.at(12, 0, 0, 0).to_zoned(TimeZone::UTC).unwrap()
    }
}

pub fn now() -> Timestamp {
    FixedClock.now().timestamp()
}

/// Series and movie root folders on disk, "Frieren (2023)" with two seasons of three episodes,
/// and "Dune (2021)".
pub struct App {
    pub dir: TempDir,
    pub db: Database,
    pub roots: RootFolders,
    pub scanner: Scanner,
    pub review: Review,
    pub renamer: Renamer,
    pub planner: ImportPlanner,
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
        let fs = Arc::new(LocalFileSystem);
        let clock = Arc::new(FixedClock);
        let roots = RootFolders::new(repo.clone(), fs.clone());
        let scanner = Scanner::new(repo.clone(), repo.clone(), fs, clock.clone());
        let review = Review::new(repo.clone(), repo.clone(), clock);
        let renamer = Renamer::new(repo.clone(), repo.clone(), Arc::new(LocalFileSystem), Naming::default());
        let planner = ImportPlanner::new(repo.clone(), repo, Arc::new(LocalFileSystem), Arc::new(FixedClock));

        let mut frieren = Series::add(frieren_metadata(), MonitorPreset::All, TODAY, now());
        let mut dune = Movie::add(dune_metadata(), true, now());
        SeriesRepo::save(&db, &mut frieren, &[]).await.unwrap();
        MovieRepo::save(&db, &mut dune, &[]).await.unwrap();

        let app = Self { dir, db, roots, scanner, review, renamer, planner, frieren, dune };
        app.roots.add(RootKind::Series, &app.path("tv")).await.unwrap();
        app.roots.add(RootKind::Movies, &app.path("movies")).await.unwrap();
        app
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
        let recorded = self.db.event_log().read_after(None, 100).await.unwrap();
        recorded.into_iter().map(|recorded| recorded.event).collect()
    }
}

pub fn episode(season: u16, episode: u16) -> EpisodeRef {
    EpisodeRef { season, episode }
}

fn frieren_metadata() -> SeriesMetadata {
    let season = |number: u16| SeasonMetadata {
        number,
        episodes: (1..=3)
            .map(|episode| EpisodeMetadata {
                source_id: u64::from(number) * 100 + u64::from(episode),
                number: episode,
                title: format!("Episode {episode}"),
                air_date: Some(date(2023, 9, 29)),
            })
            .collect(),
    };
    SeriesMetadata {
        source: ExternalId::Tmdb(209867),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        year: Some(2023),
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: vec![season(1), season(2)],
    }
}

fn dune_metadata() -> MovieMetadata {
    MovieMetadata {
        source: ExternalId::Tmdb(438631),
        title: "Dune".into(),
        original_title: "Dune".into(),
        year: Some(2021),
        poster_path: None,
        releases: Releases::default(),
    }
}

pub fn relative<'a>(app: &App, paths: impl IntoIterator<Item = &'a Path>) -> Vec<String> {
    paths.into_iter().map(|path| path.strip_prefix(app.dir.path()).unwrap().display().to_string()).collect()
}

impl App {
    pub async fn db_files(&self) -> Vec<yokoku_media::MediaFile> {
        yokoku_media::ports::MediaRepo::files(&self.db).await.unwrap()
    }
}

impl App {
    pub fn importer(&self, mode: yokoku_media::ImportMode) -> yokoku_media::Importer {
        let repo = Arc::new(self.db.clone());
        yokoku_media::Importer::new(
            repo.clone(),
            repo,
            Arc::new(LocalFileSystem),
            Arc::new(FixedClock),
            Naming::default(),
            mode,
        )
    }
}

impl App {
    pub fn deleter(&self, recycle: Option<yokoku_media::Recycle>) -> yokoku_media::Deleter {
        yokoku_media::Deleter::new(Arc::new(self.db.clone()), Arc::new(LocalFileSystem), Arc::new(FixedClock), recycle)
    }

    pub fn recycle(&self, keep_days: u32) -> Option<yokoku_media::Recycle> {
        Some(yokoku_media::Recycle { folder: self.path("recycle"), keep_days })
    }
}
