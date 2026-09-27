use std::{fs, path::Path, sync::Arc};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use jiff::{Timestamp, tz::TimeZone};
use tempfile::TempDir;
use tokio::runtime::Runtime;
use yokoku_db::Database;
use yokoku_domain::{
    EpisodeMetadata, EpisodeRef, EpisodeSpan, ExternalId, FileTarget, ImportId, ItemFolder, Live, MediaFileId,
    MonitorPreset, Movie, MovieMetadata, Releases, SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};
use yokoku_events::Publisher;
use yokoku_library::{
    Library, LibraryFilter, LibrarySort,
    ports::{MovieRepo, SeriesRepo},
};
use yokoku_media::{
    Import, ImportRow, ImportStatus, MediaFile, ScanReport, Scanner,
    ports::{Changes, MediaRepo},
};
use yokoku_system::{FileSpool, LocalFileSystem, LockFile, SystemClock};

const SEASONS: u16 = 2;
const EPISODES: u16 = 10;
const IMPORTS_PER_ITEM: usize = 2;
const SIZES: [usize; 2] = [100, 1000];

/// A library of `items` items, half series and half movies, every episode and movie with a file
/// on disk, and finished imports as history.
struct Seeded {
    dir: TempDir,
    db: Database,
}

impl Seeded {
    async fn new(items: usize) -> Self {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("yokoku.db")).await.unwrap();
        let (tv, movies) = (dir.path().join("tv"), dir.path().join("movies"));
        let now = Timestamp::now();
        let today = now.to_zoned(TimeZone::UTC).date();
        let mut changes = Changes::default();

        for number in 0..items / 2 {
            let folder = ItemFolder::new(tv.clone(), format!("Series {number:04} (2020)")).unwrap();
            let mut series = Series::add(series_metadata(number), folder, MonitorPreset::All, today, now);
            for season in 1..=SEASONS {
                for episode in 1..=EPISODES {
                    let span = EpisodeSpan::single(EpisodeRef { season, episode });
                    let path = series.folder.path().join(format!("Season {season:02}/S{season:02}E{episode:02}.mkv"));
                    let file = media_file(&path, FileTarget::Episodes { series: series.id, span });
                    series.episode_mut(EpisodeRef { season, episode }).unwrap().file = Some(file.id);
                    changes.added_files.push(file);
                }
            }
            SeriesRepo::save(&db, &mut series).await.unwrap();
        }
        for number in 0..items / 2 {
            let folder = ItemFolder::new(movies.clone(), format!("Movie {number:04} (2020)")).unwrap();
            let mut movie = Movie::add(movie_metadata(number), folder, true, now);
            let file = media_file(&movie.folder.path().join("movie.mkv"), FileTarget::Movie(movie.id));
            movie.file = Some(file.id);
            changes.added_files.push(file);
            MovieRepo::save(&db, &mut movie).await.unwrap();
        }
        for number in 0..items * IMPORTS_PER_ITEM {
            let path = dir.path().join(format!("downloads/{number}/video.mkv"));
            changes.imports.push(finished_import(&path, now));
        }
        MediaRepo::save(&db, &changes).await.unwrap();
        Self { dir, db }
    }

    fn library(&self) -> Library {
        let db = Arc::new(self.db.clone());
        Library::new(db.clone(), db, Arc::new(SystemClock::new(Live::fixed(TimeZone::UTC))), self.publisher())
    }

    fn scanner(&self) -> Scanner {
        let db = Arc::new(self.db.clone());
        let lock = Arc::new(LockFile::new(self.dir.path().join("yokoku.lock")));
        let clock = Arc::new(SystemClock::new(Live::fixed(TimeZone::UTC)));
        Scanner::new(db.clone(), db, Arc::new(LocalFileSystem), lock, clock, self.publisher())
    }

    fn publisher(&self) -> Publisher {
        Publisher::new(Arc::new(self.db.event_log()), Arc::new(FileSpool::new(self.dir.path().join("yokoku.spool"))))
    }
}

fn series_metadata(number: usize) -> SeriesMetadata {
    let source = 1_000_000 + u64::try_from(number).unwrap() * 100;
    SeriesMetadata {
        source: ExternalId::Tmdb(source),
        title: format!("Series {number:04}"),
        original_title: format!("Series {number:04}"),
        alternate_titles: Vec::new(),
        year: Some(2020),
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: (1..=SEASONS)
            .map(|season| SeasonMetadata {
                number: season,
                episodes: (1..=EPISODES)
                    .map(|episode| EpisodeMetadata {
                        source_id: source + u64::from(season) * 20 + u64::from(episode),
                        number: episode,
                        title: format!("Episode {episode}"),
                        air_date: Some(jiff::civil::date(2020, 1, 1)),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn movie_metadata(number: usize) -> MovieMetadata {
    MovieMetadata {
        source: ExternalId::Tmdb(u64::try_from(number).unwrap() + 1),
        title: format!("Movie {number:04}"),
        original_title: format!("Movie {number:04}"),
        alternate_titles: Vec::new(),
        year: Some(2020),
        poster_path: None,
        releases: Releases::default(),
    }
}

/// A one-byte file at `path` and its library record.
fn media_file(path: &Path, target: FileTarget) -> MediaFile {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, [0]).unwrap();
    MediaFile { id: MediaFileId::generate(), path: path.to_owned(), size: 1, target, added_at: Timestamp::now() }
}

fn finished_import(video: &Path, now: Timestamp) -> Import {
    let row = ImportRow {
        path: video.to_owned(),
        size: 1,
        target: None,
        confidence: yokoku_domain::Confidence::Certain,
        skipped: false,
        replace: false,
    };
    Import {
        id: ImportId::generate(),
        source: video.parent().unwrap().to_owned(),
        download: None,
        status: ImportStatus::Done,
        error: None,
        rows: vec![row],
        created_at: now,
    }
}

fn bench_library(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("library");
    group.sample_size(10);
    for items in SIZES {
        let seeded = runtime.block_on(Seeded::new(items));
        let (library, scanner) = (seeded.library(), seeded.scanner());
        assert_eq!(runtime.block_on(scanner.scan()).unwrap(), ScanReport::default(), "seeded library is in step");

        group.bench_with_input(BenchmarkId::new("list", items), &items, |b, _| {
            b.iter(|| runtime.block_on(library.list(LibraryFilter::default(), LibrarySort::default())).unwrap());
        });
        group.bench_with_input(BenchmarkId::new("scan", items), &items, |b, _| {
            b.iter(|| runtime.block_on(scanner.scan()).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_library);
criterion_main!(benches);
