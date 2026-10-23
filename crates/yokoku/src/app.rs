use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use yokoku_core::{
    downloads::{Downloads, ReleaseSearch, SyncActiveDownloads},
    events::{EventLog, History, Publisher, QueueChanges, Subscriber},
    integrations::{Ratings, Rescans, WatchSync},
    jobs::Job,
    library::{Artworks, Calendar, FileTracker, Library, MetadataService},
    media::{
        Deleter, ImportPlanner, Importer, Prober, Renamer, Reviewer, RootFolders, Scanner,
        ports::{FileSystem, LibraryLock},
    },
};
use yokoku_domain::Clock;
use yokoku_infra::{
    db::Database,
    download_clients::TransmissionClient,
    indexers::JackettClient,
    media_servers::JellyfinClient,
    metadata::{ArtworkFetcher, Sources, TmdbClient, TvdbClient},
    ratings::ImdbDataset,
    system::{ArtworkFiles, FfMpeg, FfProbe, LocalFileSystem, LockFile, SystemClock},
};

use crate::{
    background::Background,
    config::{Config, Settings},
};

/// The use cases the web UI, the CLI and jobs call, wired to their adapters, each reading the settings
/// in effect when it runs.
#[derive(Clone)]
pub struct App {
    pub settings: Settings,
    pub library: Arc<Library>,
    pub artworks: Arc<Artworks>,
    pub calendar: Arc<Calendar>,
    pub roots: Arc<RootFolders>,
    pub scanner: Arc<Scanner>,
    pub reviewer: Arc<Reviewer>,
    pub renamer: Arc<Renamer>,
    pub downloads: Arc<Downloads>,
    pub releases: Arc<ReleaseSearch>,
    pub importer: Arc<Importer>,
    pub history: Arc<History>,
    pub clock: Arc<dyn Clock>,
    /// Wakes after this process changes downloads or imports.
    pub queue_changes: QueueChanges,
    pub deleter: Arc<Deleter>,
    pub prober: Arc<Prober>,
    pub rescans: Arc<Rescans>,
    pub ratings: Arc<Ratings>,
    pub metadata: Arc<MetadataService>,
}

impl App {
    /// `config` comes from the config file at `config_path` and the environment; the stored
    /// settings go over it once the database is open. Returns the app with what runs in the background.
    pub async fn open(config: &Config, config_path: Option<&Path>) -> Result<(Self, Background)> {
        config.validate().context("Invalid configuration")?;
        let path = &config.database.path;
        let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
        let db = Arc::new(db);
        let settings = Settings::open(config_path.map(Path::to_path_buf), db.clone())
            .await
            .context("Failed to load configuration with the stored settings; see `yokoku settings list`")?;
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new(settings.live(|config| config.clock.time_zone())));
        let log = EventLog::new(Database::clone(&db));
        let events = Publisher::new(log.clone());
        let queue_changes = QueueChanges::new();
        let naming = settings.live(|config| config.naming.clone());
        let metadata_settings = settings.live(|config| config.metadata.clone());
        let metadata = Arc::new(MetadataService::new(
            db.clone(),
            db.clone(),
            Arc::new(Sources::new(
                Arc::new(TmdbClient::new(metadata_settings.clone())),
                Arc::new(TvdbClient::new(metadata_settings)),
                settings.live(|config| config.metadata.tvdb.api_key.is_some()),
            )),
            naming.clone(),
            clock.clone(),
            events.clone(),
        ));

        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let roots = Arc::new(RootFolders::new(
            db.clone(),
            db.clone(),
            fs.clone(),
            config.roots.iter().cloned().map(Into::into).collect(),
        ));
        let lock: Arc<dyn LibraryLock> = Arc::new(LockFile::new(path.with_extension("lock")));
        let probe = FfProbe::new(settings.live(|config| config.files.ffprobe.clone()));
        let prober = Arc::new(Prober::new(db.clone(), fs.clone(), Arc::new(probe)));
        let deleter = Arc::new(Deleter::new(db.clone(), roots.clone(), fs.clone(), lock.clone(), events.clone()));
        let jellyfin = Arc::new(JellyfinClient::new(settings.live(|config| config.jellyfin.clone())));
        let rescans = Arc::new(Rescans::new(db.clone(), jellyfin.clone(), clock.clone()));
        let ratings = Arc::new(Ratings::new(
            Arc::new(ImdbDataset::new(settings.live(|config| config.ratings.clone()), path.with_file_name("ratings"))),
            db.clone(),
            db.clone(),
        ));
        let downloads = Arc::new(Downloads::new(
            db.clone(),
            Arc::new(TransmissionClient::new(settings.live(|config| config.transmission.clone()))),
            clock.clone(),
            settings.live(|config| config.downloads.clone()),
            events.clone(),
            queue_changes.clone(),
        ));
        let scanner = Arc::new(Scanner::new(
            db.clone(),
            db.clone(),
            fs.clone(),
            lock.clone(),
            clock.clone(),
            events.clone(),
            queue_changes.clone(),
        ));
        let artworks = Arc::new(Artworks::new(
            db.clone(),
            db.clone(),
            Arc::new(ArtworkFetcher::new()),
            Arc::new(ArtworkFiles::new(path.with_file_name("artwork"))),
        ));

        let importer = Arc::new(Importer::new(
            db.clone(),
            db.clone(),
            fs.clone(),
            lock.clone(),
            clock.clone(),
            naming.clone(),
            settings.live(|config| config.import.clone()),
            Arc::new(FfMpeg::new(
                settings.live(|config| config.files.ffmpeg.clone()),
                settings.live(|config| config.files.ffprobe.clone()),
            )),
            events.clone(),
            queue_changes.clone(),
        ));

        let subscribers: Vec<Arc<dyn Subscriber>> = vec![
            scanner.clone(),
            ratings.clone(),
            Arc::new(FileTracker::new(db.clone(), db.clone(), db.clone())),
            artworks.clone(),
            Arc::new(ImportPlanner::new(
                db.clone(),
                db.clone(),
                fs.clone(),
                clock.clone(),
                events.clone(),
                queue_changes.clone(),
            )),
            deleter.clone(),
            downloads.clone(),
            prober.clone(),
            rescans.clone(),
            Arc::new(settings.clone()),
        ];
        let jobs: Vec<Arc<dyn Job>> = vec![
            downloads.clone(),
            Arc::new(SyncActiveDownloads::new(downloads.clone())),
            importer.clone(),
            scanner.clone(),
            metadata.clone(),
            rescans.clone(),
            Arc::new(WatchSync::new(jellyfin, db.clone(), db.clone(), db.clone())),
            ratings.clone(),
        ];

        let app = Self {
            library: Arc::new(Library::new(db.clone(), db.clone(), db.clone(), clock.clone(), events.clone())),
            calendar: Arc::new(Calendar::new(db.clone(), db.clone(), clock.clone())),
            roots: roots.clone(),
            scanner,
            reviewer: Arc::new(Reviewer::new(
                db.clone(),
                db.clone(),
                clock.clone(),
                events.clone(),
                queue_changes.clone(),
            )),
            releases: Arc::new(ReleaseSearch::new(
                Arc::new(JackettClient::new(settings.live(|config| config.jackett.clone()))),
                downloads.clone(),
            )),
            downloads,
            renamer: Arc::new(Renamer::new(db.clone(), db.clone(), roots, fs, lock, naming, events.clone())),
            importer,
            history: Arc::new(History::new(log.clone())),
            clock,
            queue_changes,
            settings: settings.clone(),
            deleter,
            prober,
            rescans,
            ratings,
            metadata,
            artworks,
        };
        let background = Background::new(log, events, subscribers, jobs, &settings)?;
        Ok((app, background))
    }
}
