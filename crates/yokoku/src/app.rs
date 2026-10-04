use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::{task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use yokoku_core::{
    downloads::Downloads,
    events::{Delivery, DeliveryConfig, EventLog, History, Publisher, QueueChanges},
    integrations::{Rescans, WatchSync},
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
    media_servers::JellyfinClient,
    metadata::{ArtworkFetcher, Sources, TmdbClient, TvdbClient},
    system::{ArtworkFiles, FfMpeg, FfProbe, LocalFileSystem, LockFile, SystemClock},
};

use crate::config::{Config, Settings};

/// Use cases wired to their adapters, each reading the settings in effect when it runs.
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
    pub importer: Arc<Importer>,
    pub history: Arc<History>,
    pub clock: Arc<dyn Clock>,
    /// Wakes after this process changes downloads or imports.
    pub queue_changes: QueueChanges,
    pub deleter: Arc<Deleter>,
    pub prober: Arc<Prober>,
    pub rescans: Arc<Rescans>,
    pub watched: Arc<WatchSync>,
    pub metadata: Arc<MetadataService>,
    pub events: Publisher,
    /// Handlers that only events reach.
    pub planner: Arc<ImportPlanner>,
    pub tracker: Arc<FileTracker>,
    log: EventLog,
}

impl App {
    /// `config` comes from the config file at `config_path` and the environment; the stored
    /// settings go over it once the database is open.
    pub async fn open(config: &Config, config_path: Option<&Path>) -> Result<Self> {
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
        let watched = Arc::new(WatchSync::new(jellyfin, db.clone(), db.clone(), db.clone()));
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

        Ok(Self {
            library: Arc::new(Library::new(db.clone(), db.clone(), db.clone(), clock.clone(), events.clone())),
            calendar: Arc::new(Calendar::new(db.clone(), db.clone(), clock.clone())),
            roots: roots.clone(),
            scanner: scanner.clone(),
            reviewer: Arc::new(Reviewer::new(
                db.clone(),
                db.clone(),
                clock.clone(),
                events.clone(),
                queue_changes.clone(),
            )),
            downloads: downloads.clone(),
            renamer: Arc::new(Renamer::new(
                db.clone(),
                db.clone(),
                roots.clone(),
                fs.clone(),
                lock.clone(),
                naming.clone(),
                events.clone(),
            )),
            importer: Arc::new(Importer::new(
                db.clone(),
                db.clone(),
                fs.clone(),
                lock,
                clock.clone(),
                naming,
                settings.live(|config| config.import.clone()),
                Arc::new(FfMpeg::new(
                    settings.live(|config| config.files.ffmpeg.clone()),
                    settings.live(|config| config.files.ffprobe.clone()),
                )),
                events.clone(),
                queue_changes.clone(),
            )),
            history: Arc::new(History::new(log.clone())),
            clock: clock.clone(),
            queue_changes: queue_changes.clone(),
            events: events.clone(),
            log,
            planner: Arc::new(ImportPlanner::new(
                db.clone(),
                db.clone(),
                fs.clone(),
                clock.clone(),
                events.clone(),
                queue_changes.clone(),
            )),
            tracker: Arc::new(FileTracker::new(db.clone(), db.clone(), db.clone())),
            settings,
            deleter,
            prober,
            rescans,
            watched,
            metadata,
            artworks,
        })
    }

    /// Starts one delivery loop per subscriber and one that appends kept events every
    /// `FLUSH_INTERVAL`; each stops when `shutdown` is cancelled.
    pub fn spawn_deliveries(&self, shutdown: &CancellationToken) -> JoinSet<()> {
        let poll_interval = Duration::from_millis(self.settings.current().events.poll_interval_ms);
        let config = DeliveryConfig { poll_interval, ..DeliveryConfig::default() };
        let mut deliveries: JoinSet<()> = self
            .subscriptions()
            .into_iter()
            .map(|subscriber| Delivery::new(self.log.clone(), subscriber, config.clone()).run(shutdown.clone()))
            .collect();
        deliveries.spawn(self.flush_kept(shutdown.clone()));
        deliveries
    }

    fn flush_kept(&self, shutdown: CancellationToken) -> impl Future<Output = ()> + Send + 'static {
        const FLUSH_INTERVAL: Duration = Duration::from_secs(60);
        let events = self.events.clone();
        async move {
            shutdown
                .run_until_cancelled(async {
                    loop {
                        if let Err(error) = events.flush().await {
                            warn!(%error, "kept events are still waiting for the event log");
                        }
                        sleep(FLUSH_INTERVAL).await;
                    }
                })
                .await;
        }
    }
}
