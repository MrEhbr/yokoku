use std::{fmt, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use jiff::{SignedDuration, tz::TimeZone};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use yokoku_db::Database;
use yokoku_domain::Clock;
use yokoku_downloads::Downloads;
use yokoku_events::{Delivery, DeliveryConfig, History, Subscriber};
use yokoku_integrations::Rescans;
use yokoku_library::{Library, MetadataSync, Schedule};
use yokoku_media::{
    Deleter, Importer, Renamer, Review, RootFolders, Scanner,
    ports::{FileSystem, LibraryLock},
};
use yokoku_metadata::TmdbClient;
use yokoku_naming::Naming;
use yokoku_system::{JellyfinClient, LocalFileSystem, LockFile, SystemClock};
use yokoku_transmission::TransmissionClient;

use crate::{config::Config, subscriptions};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: PathBuf::from("yokoku.db") }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct ClockConfig {
    /// IANA time zone name, e.g. `Europe/Berlin`; the system zone when unset.
    pub timezone: Option<String>,
}

impl ClockConfig {
    pub fn time_zone(&self) -> Result<TimeZone> {
        match &self.timezone {
            Some(name) => TimeZone::get(name).with_context(|| format!("Unknown time zone: {name}")),
            None => Ok(TimeZone::system()),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct MetadataConfig {
    /// TMDB API read access token; set it through `APP__METADATA__TMDB_TOKEN`.
    pub tmdb_token: Option<String>,
    pub tmdb_url: String,
    pub language: String,
    /// Country whose movie release dates are used.
    pub region: String,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            tmdb_token: None,
            tmdb_url: "https://api.themoviedb.org/3".into(),
            language: "en-US".into(),
            region: "US".into(),
        }
    }
}

impl fmt::Debug for MetadataConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetadataConfig")
            .field("tmdb_token", &self.tmdb_token.as_ref().map(|_| "<redacted>"))
            .field("tmdb_url", &self.tmdb_url)
            .field("language", &self.language)
            .field("region", &self.region)
            .finish()
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct TransmissionConfig {
    pub url: String,
    pub username: Option<String>,
    /// Set it through `APP__TRANSMISSION__PASSWORD`.
    pub password: Option<String>,
}

impl Default for TransmissionConfig {
    fn default() -> Self {
        Self { url: "http://localhost:9091/transmission/rpc".into(), username: None, password: None }
    }
}

impl fmt::Debug for TransmissionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransmissionConfig")
            .field("url", &self.url)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// Use cases wired to their adapters.
pub struct App {
    pub library: Library,
    pub schedule: Schedule,
    pub roots: RootFolders,
    pub scanner: Arc<Scanner>,
    pub review: Review,
    pub renamer: Renamer,
    pub downloads: Arc<Downloads>,
    pub importer: Arc<Importer>,
    pub history: History,
    pub deleter: Arc<Deleter>,
    /// `None` while no Jellyfin is configured.
    pub rescans: Option<Arc<Rescans>>,
    sync: Option<Arc<MetadataSync>>,
    db: Arc<Database>,
    subscribers: Vec<Arc<dyn Subscriber>>,
}

impl App {
    pub async fn open(config: &Config) -> Result<Self> {
        let path = &config.database.path;
        let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
        let db = Arc::new(db);
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new(config.clock.time_zone()?));
        let metadata = &config.metadata;
        let sync = metadata.tmdb_token.as_ref().map(|token| {
            let tmdb = TmdbClient::new(token, &metadata.language, &metadata.region).with_base_url(&metadata.tmdb_url);
            Arc::new(MetadataSync::new(db.clone(), db.clone(), Arc::new(tmdb), clock.clone()))
        });

        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let lock: Arc<dyn LibraryLock> = Arc::new(LockFile::new(path.with_extension("lock")));
        let deleter =
            Arc::new(Deleter::new(db.clone(), fs.clone(), lock.clone(), clock.clone(), config.recycle.recycle()));
        let jellyfin = &config.jellyfin;
        let rescans = jellyfin.url.as_ref().map(|url| {
            let server = JellyfinClient::new(url, jellyfin.api_key.clone().unwrap_or_default());
            Arc::new(Rescans::new(db.clone(), Arc::new(server), clock.clone()))
        });
        let transmission = &config.transmission;
        let mut client = TransmissionClient::new(&transmission.url);
        if let Some(username) = &transmission.username {
            client = client.with_credentials(username, transmission.password.clone().unwrap_or_default());
        }
        let downloads = Arc::new(Downloads::new(db.clone(), Arc::new(client), clock.clone()));

        Ok(Self {
            library: Library::new(db.clone(), db.clone(), clock.clone()),
            schedule: Schedule::new(db.clone(), db.clone(), clock.clone()),
            roots: RootFolders::new(db.clone(), fs.clone()),
            scanner: Arc::new(Scanner::new(db.clone(), db.clone(), fs.clone(), lock.clone(), clock.clone())),
            review: Review::new(db.clone(), db.clone(), clock.clone()),
            downloads: downloads.clone(),
            renamer: Renamer::new(db.clone(), db.clone(), fs.clone(), lock.clone(), Naming::default()),
            importer: Arc::new(Importer::new(
                db.clone(),
                db.clone(),
                fs.clone(),
                lock,
                clock.clone(),
                Naming::default(),
                config.import.mode.into(),
            )),
            history: History::new(Arc::new(db.event_log())),
            subscribers: subscriptions::subscribers(&db, &fs, &clock, &deleter, &downloads, rescans.as_ref()),
            deleter,
            rescans,
            sync,
            db,
        })
    }

    /// Delivers pending events to every subscriber, then asks Jellyfin to rescan if they changed
    /// library files; a Jellyfin that cannot be reached is only reported.
    pub async fn deliver_events(&self) -> Result<()> {
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

    /// Starts one delivery loop per subscriber; each stops when `shutdown` is cancelled.
    pub fn spawn_deliveries(&self, shutdown: &CancellationToken) -> Vec<JoinHandle<()>> {
        self.subscribers
            .iter()
            .map(|subscriber| {
                let log = Arc::new(self.db.event_log());
                let delivery =
                    Delivery::new(log, subscriber.clone(), self.db.new_events().listen(), DeliveryConfig::default());
                tokio::spawn(delivery.run(shutdown.clone()))
            })
            .collect()
    }

    /// Use cases that need the metadata source.
    pub fn sync(&self) -> Result<&MetadataSync> {
        self.sync.as_deref().context("No TMDB token configured; set APP__METADATA__TMDB_TOKEN")
    }

    /// `None` while no TMDB token is configured.
    pub fn metadata(&self) -> Option<Arc<MetadataSync>> {
        self.sync.clone()
    }
}

/// Gives up after three quick attempts; the event is tried again on the next catch-up or by `serve`.
fn quick_delivery() -> DeliveryConfig {
    DeliveryConfig {
        max_attempts: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(1),
        ..DeliveryConfig::default()
    }
}
