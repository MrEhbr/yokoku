use std::{fs, path::Path, sync::Arc, time::Duration};

use async_trait::async_trait;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main, profiler::Profiler};
use jiff::{Timestamp, tz::TimeZone};
use pprof::{ProfilerGuard, protos::Message};
use tempfile::TempDir;
use tokio::runtime::Runtime;
use yokoku_core::{
    events::{EventLog, Publisher, QueueChanges},
    library::{
        Library, LibraryFilter, LibrarySort,
        ports::{MovieRepo, SeriesRepo},
    },
    media::{
        AudioStream, Import, ImportRow, ImportStatus, MediaFile, MediaInfo, Prober, Resolution, RowMatch, ScanReport,
        Scanner, SubtitleStream, VideoStream,
        ports::{Changes, MediaProbe, MediaRepo, ProbeError},
    },
};
use yokoku_domain::{
    Artwork, Description, EpisodeMetadata, EpisodeRef, EpisodeSpan, ExternalId, FileTarget, ImportId, ItemFolder,
    ItemId, Live, MediaFileId, MonitorPreset, Movie, MovieMetadata, Releases, SeasonMetadata, Series, SeriesId,
    SeriesMetadata, SourceStatus,
};
use yokoku_infra::{
    db::Database,
    system::{LocalFileSystem, LockFile, SystemClock},
};

const SEASONS: u16 = 2;
const EPISODES: u16 = 10;
const IMPORTS_PER_ITEM: usize = 2;
const SIZES: [usize; 2] = [100, 1000];
const LONG_SEASONS: u16 = 12;
const EPISODES_PER_SEASON: [u16; 2] = [10, 100];

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
            let mut series = Series::new(series_metadata(number), folder, MonitorPreset::All, today, now);
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
            let mut movie = Movie::new(movie_metadata(number), folder, true, now);
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
        Scanner::new(db.clone(), db, Arc::new(LocalFileSystem), lock, clock, self.publisher(), QueueChanges::new())
    }

    fn publisher(&self) -> Publisher {
        Publisher::new(EventLog::new(self.db.clone()))
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
        artwork: Artwork::default(),
        description: Description::default(),
        status: SourceStatus::Returning,
        seasons: (1..=SEASONS)
            .map(|season| SeasonMetadata {
                number: season,
                episodes: (1..=EPISODES)
                    .map(|episode| EpisodeMetadata {
                        source_id: source + u64::from(season) * 20 + u64::from(episode),
                        number: episode,
                        title: format!("Episode {episode}"),
                        overview: String::new(),
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
        artwork: Artwork::default(),
        description: Description::default(),
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
        matched: RowMatch::None,
        confidence: yokoku_domain::Confidence::Certain,
        skipped: false,
        resolution: Resolution::Unresolved,
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

/// One series' files, `seasons` folders of `episodes` each, every video with a subtitle beside it
/// and stored media info.
struct LongSeries {
    _dir: TempDir,
    db: Database,
    series: SeriesId,
}

impl LongSeries {
    async fn new(seasons: u16, episodes: u16) -> Self {
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("yokoku.db")).await.unwrap();
        let series = SeriesId::generate();
        let mut changes = Changes::default();
        for season in 1..=seasons {
            for episode in 1..=episodes {
                let path = dir.path().join(format!("tv/Long (2020)/Season {season:02}/S{season:02}E{episode:03}.mkv"));
                let span = EpisodeSpan::single(EpisodeRef { season, episode });
                changes.added_files.push(media_file(&path, FileTarget::Episodes { series, span }));
                fs::write(path.with_extension("en.srt"), [0]).unwrap();
            }
        }
        MediaRepo::save(&db, &changes).await.unwrap();
        for file in &changes.added_files {
            db.save_media_info(file.id, &full_hd()).await.unwrap();
        }
        Self { _dir: dir, db, series }
    }

    fn prober(&self) -> Prober {
        Prober::new(Arc::new(self.db.clone()), Arc::new(LocalFileSystem), Arc::new(NoProbe))
    }
}

fn full_hd() -> MediaInfo {
    MediaInfo {
        duration: Some(Duration::from_secs(1440)),
        video: Some(VideoStream { codec: "h264".into(), width: 1920, height: 1080 }),
        audio: vec![AudioStream { codec: "aac".into(), language: Some("jpn".into()), channels: 2 }],
        subtitles: vec![SubtitleStream { codec: "ass".into(), language: Some("eng".into()), forced: false }],
    }
}

/// A probe that is never installed.
struct NoProbe;

#[async_trait]
impl MediaProbe for NoProbe {
    async fn probe(&self, _path: &Path) -> Result<MediaInfo, ProbeError> {
        Err(ProbeError::Missing)
    }
}

fn bench_prober(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let mut group = c.benchmark_group("prober");
    group.sample_size(10);
    for episodes in EPISODES_PER_SEASON {
        let long = runtime.block_on(LongSeries::new(LONG_SEASONS, episodes));
        let (prober, item) = (long.prober(), ItemId::Series(long.series));
        let files = usize::from(LONG_SEASONS * episodes);
        group.bench_with_input(BenchmarkId::new("details", files), &files, |b, &files| {
            b.iter(|| assert_eq!(runtime.block_on(prober.details(item)).unwrap().len(), files));
        });
    }
    group.finish();
}

/// Samples a benchmark run with `--profile-time` and writes `profile.pb` beside its report.
struct Pprof(Option<ProfilerGuard<'static>>);

impl Profiler for Pprof {
    fn start_profiling(&mut self, _id: &str, _dir: &Path) {
        self.0 = Some(ProfilerGuard::new(1000).unwrap());
    }

    fn stop_profiling(&mut self, _id: &str, dir: &Path) {
        let Some(guard) = self.0.take() else { return };
        let profile = guard.report().build().unwrap().pprof().unwrap();
        let mut content = Vec::new();
        profile.write_to_vec(&mut content).unwrap();
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("profile.pb"), content).unwrap();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(Pprof(None));
    targets = bench_library, bench_prober
}
criterion_main!(benches);
