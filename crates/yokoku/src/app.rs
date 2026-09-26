use std::{fmt, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use yokoku_db::Database;
use yokoku_library::{Library, MetadataSync, Schedule};
use yokoku_metadata::TmdbClient;
use yokoku_system::SystemClock;

use crate::config::Config;

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
    fn time_zone(&self) -> Result<TimeZone> {
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

/// Use cases wired to their adapters.
pub struct App {
    pub library: Library,
    pub schedule: Schedule,
    sync: Option<MetadataSync>,
}

impl App {
    pub async fn open(config: &Config) -> Result<Self> {
        let path = &config.database.path;
        let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
        let db = Arc::new(db);
        let clock = Arc::new(SystemClock::new(config.clock.time_zone()?));
        let metadata = &config.metadata;
        let sync = metadata.tmdb_token.as_ref().map(|token| {
            let tmdb = TmdbClient::new(token, &metadata.language, &metadata.region).with_base_url(&metadata.tmdb_url);
            MetadataSync::new(db.clone(), db.clone(), Arc::new(tmdb), clock.clone())
        });

        Ok(Self {
            library: Library::new(db.clone(), db.clone(), clock.clone()),
            schedule: Schedule::new(db.clone(), db, clock),
            sync,
        })
    }

    /// Use cases that need the metadata source.
    pub fn sync(&self) -> Result<&MetadataSync> {
        self.sync.as_ref().context("No TMDB token configured; set APP__METADATA__TMDB_TOKEN")
    }
}
