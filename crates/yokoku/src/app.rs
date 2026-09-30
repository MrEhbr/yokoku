use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use yokoku_config::{Config, Settings};
use yokoku_db::Database;
use yokoku_domain::{Clock, ItemId, Live, title_with_year};
use yokoku_download_clients::TransmissionClient;
use yokoku_downloads::Downloads;
use yokoku_events::{Delivery, DeliveryConfig, EventLog, History, Publisher, QueueChanges, Subscription};
use yokoku_integrations::Rescans;
use yokoku_library::{Artworks, Calendar, Library, MetadataService, ports::FolderNames};
use yokoku_media::{
    Deleter, ImportPlanner, Importer, Prober, Renamer, Reviewer, RootFolders, Scanner,
    ports::{FileSystem, LibraryLock},
};
use yokoku_media_servers::JellyfinClient;
use yokoku_metadata::{ArtworkFetcher, Sources, TmdbClient, TvdbClient};
use yokoku_naming::Naming;
use yokoku_system::{ArtworkFiles, FfProbe, LocalFileSystem, LockFile, SystemClock};

use crate::subscriptions;

/// Use cases wired to their adapters, each reading the settings in effect when it runs.
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
    metadata: Arc<MetadataService>,
    pub events: Publisher,
    log: EventLog,
    subscribers: Vec<Arc<Subscription>>,
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
        let log = EventLog::new(db.pool().clone());
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
            Arc::new(NamedFolders(naming.clone())),
            clock.clone(),
            events.clone(),
        ));

        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let lock: Arc<dyn LibraryLock> = Arc::new(LockFile::new(path.with_extension("lock")));
        let probe = FfProbe::new(settings.live(|config| config.files.ffprobe.clone()));
        let prober = Arc::new(Prober::new(db.clone(), fs.clone(), Arc::new(probe)));
        let deleter = Arc::new(Deleter::new(db.clone(), fs.clone(), lock.clone(), events.clone()));
        let jellyfin = JellyfinClient::new(settings.live(|config| config.jellyfin.clone()));
        let rescans = Arc::new(Rescans::new(db.clone(), Arc::new(jellyfin), clock.clone()));
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
            library: Arc::new(Library::new(db.clone(), db.clone(), clock.clone(), events.clone())),
            calendar: Arc::new(Calendar::new(db.clone(), db.clone(), clock.clone())),
            roots: Arc::new(RootFolders::new(db.clone(), db.clone(), fs.clone())),
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
                settings.live(|config| config.import.mode),
                events.clone(),
                queue_changes.clone(),
            )),
            history: Arc::new(History::new(log.clone())),
            clock: clock.clone(),
            queue_changes: queue_changes.clone(),
            events: events.clone(),
            log,
            subscribers: subscriptions::subscribers(
                &db,
                &Arc::new(ImportPlanner::new(
                    db.clone(),
                    db.clone(),
                    fs.clone(),
                    clock.clone(),
                    events.clone(),
                    queue_changes.clone(),
                )),
                &deleter,
                &downloads,
                &prober,
                &scanner,
                &rescans,
                &artworks,
                &settings,
            ),
            settings,
            deleter,
            prober,
            rescans,
            metadata,
            artworks,
        })
    }

    /// Starts one delivery loop per subscriber and one that appends kept events every
    /// `FLUSH_INTERVAL`; each stops when `shutdown` is cancelled.
    pub fn spawn_deliveries(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<()>> {
        let poll_interval = Duration::from_millis(self.settings.current().events.poll_interval_ms);
        let config = DeliveryConfig { poll_interval, ..DeliveryConfig::default() };
        self.subscribers
            .iter()
            .map(|subscriber| {
                let delivery = Delivery::new(self.log.clone(), subscriber.clone(), config.clone());
                tokio::spawn(delivery.run(shutdown.clone()))
            })
            .chain([self.spawn_flush(shutdown)])
            .collect()
    }

    fn spawn_flush(&self, shutdown: &CancellationToken) -> JoinHandle<()> {
        let (events, shutdown) = (self.events.clone(), shutdown.clone());
        tokio::spawn(async move {
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
        })
    }

    /// Use cases that need the metadata source; fails while no TMDB token is configured.
    pub fn metadata(&self) -> Result<&MetadataService> {
        if self.settings.current().metadata.tmdb.token.is_none() {
            bail!("No TMDB token configured; set APP__METADATA__TMDB__TOKEN");
        }
        Ok(&self.metadata)
    }

    pub fn metadata_service(&self) -> Arc<MetadataService> {
        self.metadata.clone()
    }

    /// The item's title with its year; `removed series` or `removed movie` once it left the library.
    pub async fn title(&self, item: ItemId) -> String {
        let found = match item {
            ItemId::Series(id) => self.library.series(id).await.map(|series| (series.title, series.year)),
            ItemId::Movie(id) => self.library.movie(id).await.map(|movie| (movie.title, movie.year)),
        };
        found.map_or_else(|_| format!("removed {}", item.kind()), |(title, year)| title_with_year(&title, year))
    }
}

/// Names new item folders with the configured naming patterns.
struct NamedFolders(Live<Naming>);

impl FolderNames for NamedFolders {
    fn series_folder(&self, title: &str, year: Option<i16>) -> String {
        self.0.current().series_folder(title, year)
    }

    fn movie_folder(&self, title: &str, year: Option<i16>) -> String {
        self.0.current().movie_folder(title, year)
    }
}

const FLUSH_INTERVAL: Duration = Duration::from_secs(60);
