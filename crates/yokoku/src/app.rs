use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use jiff::SignedDuration;
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use yokoku_config::Config;
use yokoku_db::Database;
use yokoku_domain::{Clock, ItemId, MovieMetadata, SeriesMetadata, title_with_year};
use yokoku_downloads::Downloads;
use yokoku_events::{Delivery, DeliveryConfig, History, Publisher, Subscriber};
use yokoku_integrations::Rescans;
use yokoku_library::{
    Calendar, Library, MetadataService,
    ports::{FolderNames, MetadataProvider},
};
use yokoku_media::{
    Deleter, ImportPlanner, Importer, Prober, Renamer, Reviewer, RootFolders, Scanner,
    ports::{FileSystem, LibraryLock},
};
use yokoku_metadata::{Sources, TmdbClient, TvdbClient};
use yokoku_naming::Naming;
use yokoku_system::{FfProbe, FileSpool, JellyfinClient, LocalFileSystem, LockFile, SystemClock};
use yokoku_transmission::TransmissionClient;

use crate::subscriptions;

/// Use cases wired to their adapters.
pub struct App {
    pub library: Library,
    pub calendar: Calendar,
    pub roots: RootFolders,
    pub scanner: Arc<Scanner>,
    pub reviewer: Reviewer,
    pub renamer: Renamer,
    pub downloads: Arc<Downloads>,
    pub importer: Arc<Importer>,
    pub history: History,
    pub deleter: Arc<Deleter>,
    pub prober: Arc<Prober>,
    /// `None` while no Jellyfin is configured.
    pub rescans: Option<Arc<Rescans>>,
    metadata: Option<Arc<MetadataService>>,
    db: Arc<Database>,
    events: Publisher,
    subscribers: Vec<Arc<dyn Subscriber>>,
}

impl App {
    pub async fn open(config: &Config) -> Result<Self> {
        let path = &config.database.path;
        let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
        let db = Arc::new(db);
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new(config.clock.time_zone()));
        let events = Publisher::new(Arc::new(db.event_log()), Arc::new(FileSpool::new(path.with_extension("spool"))));
        let naming = config.naming.clone();
        let settings = &config.metadata;
        let tvdb_language = settings.tvdb_language()?;
        let metadata = settings.tmdb.token.as_ref().map(|token| {
            let tmdb =
                TmdbClient::new(token.expose(), &settings.language, &settings.region).with_base_url(&settings.tmdb.url);
            let tvdb = settings.tvdb.api_key.as_ref().map(|key| {
                let pin = settings.tvdb.pin.as_ref().map(|pin| pin.expose().to_owned());
                let client = TvdbClient::new(key.expose(), pin, tvdb_language);
                Arc::new(client.with_base_url(&settings.tvdb.url)) as Arc<dyn MetadataProvider>
            });
            let folders = Arc::new(NamedFolders(naming.clone()));
            Arc::new(MetadataService::new(
                db.clone(),
                db.clone(),
                Arc::new(Sources::new(Arc::new(tmdb), tvdb)),
                folders,
                clock.clone(),
                events.clone(),
            ))
        });

        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let lock: Arc<dyn LibraryLock> = Arc::new(LockFile::new(path.with_extension("lock")));
        let prober = Arc::new(Prober::new(db.clone(), fs.clone(), Arc::new(FfProbe::new(&config.files.ffprobe))));
        let deleter = Arc::new(Deleter::new(db.clone(), fs.clone(), lock.clone(), events.clone()));
        let jellyfin = &config.jellyfin;
        let rescans = jellyfin.url.as_ref().map(|url| {
            let server = JellyfinClient::new(url, jellyfin.api_key.as_ref().map_or("", |key| key.expose()));
            Arc::new(Rescans::new(db.clone(), Arc::new(server), clock.clone()))
        });
        let transmission = &config.transmission;
        let mut client = TransmissionClient::new(&transmission.url);
        if let Some(username) = &transmission.username {
            let password = transmission.password.as_ref().map_or("", |password| password.expose());
            client = client.with_credentials(username, password);
        }
        let downloads = Arc::new(Downloads::new(
            db.clone(),
            Arc::new(client),
            clock.clone(),
            config.downloads.clone(),
            events.clone(),
        ));
        let scanner =
            Arc::new(Scanner::new(db.clone(), db.clone(), fs.clone(), lock.clone(), clock.clone(), events.clone()));

        Ok(Self {
            library: Library::new(db.clone(), db.clone(), clock.clone(), events.clone()),
            calendar: Calendar::new(db.clone(), db.clone(), clock.clone()),
            roots: RootFolders::new(db.clone(), db.clone(), fs.clone()),
            scanner: scanner.clone(),
            reviewer: Reviewer::new(db.clone(), db.clone(), clock.clone(), events.clone()),
            downloads: downloads.clone(),
            renamer: Renamer::new(db.clone(), db.clone(), fs.clone(), lock.clone(), naming.clone(), events.clone()),
            importer: Arc::new(Importer::new(
                db.clone(),
                db.clone(),
                fs.clone(),
                lock,
                clock.clone(),
                naming,
                config.import.mode,
                events.clone(),
            )),
            history: History::new(Arc::new(db.event_log())),
            events: events.clone(),
            subscribers: subscriptions::subscribers(
                &db,
                &Arc::new(ImportPlanner::new(db.clone(), db.clone(), fs.clone(), clock.clone(), events.clone())),
                &deleter,
                &downloads,
                &prober,
                &scanner,
                rescans.as_ref(),
            ),
            deleter,
            prober,
            rescans,
            metadata,
            db,
        })
    }

    /// Appends spooled events, delivers pending events to every subscriber, then asks Jellyfin to
    /// rescan if they changed library files; a Jellyfin that cannot be reached is only reported.
    pub async fn deliver_events(&self) -> Result<()> {
        self.events.replay().await;
        for subscriber in &self.subscribers {
            let log = Arc::new(self.db.event_log());
            let delivery = Delivery::new(log, subscriber.clone(), self.db.new_events().listen(), quick_delivery());
            delivery.catch_up().await.context("Failed to deliver events")?;
        }
        if let Some(rescans) = &self.rescans
            && let Err(error) = rescans.run_due(SignedDuration::ZERO).await
        {
            warn!(%error, "Jellyfin rescan failed; it will be tried again");
        }
        Ok(())
    }

    /// Starts one delivery loop per subscriber and one that appends spooled events every
    /// `REPLAY_INTERVAL`; each stops when `shutdown` is cancelled.
    pub fn spawn_deliveries(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<()>> {
        self.subscribers
            .iter()
            .map(|subscriber| {
                let log = Arc::new(self.db.event_log());
                let delivery =
                    Delivery::new(log, subscriber.clone(), self.db.new_events().listen(), DeliveryConfig::default());
                tokio::spawn(delivery.run(shutdown.clone()))
            })
            .chain([self.spawn_replay(shutdown)])
            .collect()
    }

    fn spawn_replay(&self, shutdown: &CancellationToken) -> JoinHandle<()> {
        let (events, shutdown) = (self.events.clone(), shutdown.clone());
        tokio::spawn(async move {
            shutdown
                .run_until_cancelled(async {
                    loop {
                        events.replay().await;
                        sleep(REPLAY_INTERVAL).await;
                    }
                })
                .await;
        })
    }

    /// Use cases that need the metadata source.
    pub fn metadata(&self) -> Result<&MetadataService> {
        self.metadata.as_deref().context("No TMDB token configured; set APP__METADATA__TMDB__TOKEN")
    }

    /// `None` while no TMDB token is configured.
    pub fn metadata_service(&self) -> Option<Arc<MetadataService>> {
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
struct NamedFolders(Naming);

impl FolderNames for NamedFolders {
    fn series_folder(&self, metadata: &SeriesMetadata) -> String {
        self.0.series_folder(&metadata.title, metadata.year)
    }

    fn movie_folder(&self, metadata: &MovieMetadata) -> String {
        self.0.movie_folder(&metadata.title, metadata.year)
    }
}

const REPLAY_INTERVAL: Duration = Duration::from_secs(60);

/// Gives up after three quick attempts; the event is tried again on the next catch-up or by `serve`.
fn quick_delivery() -> DeliveryConfig {
    DeliveryConfig {
        max_attempts: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(1),
        ..DeliveryConfig::default()
    }
}
