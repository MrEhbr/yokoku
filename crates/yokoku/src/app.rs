use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use yokoku_db::Database;
use yokoku_library::Library;
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

/// Use cases wired to their adapters.
pub struct App {
    pub library: Library,
}

impl App {
    pub async fn open(config: &Config) -> Result<Self> {
        let path = &config.database.path;
        let db = Database::open(path).await.with_context(|| format!("Failed to open database: {}", path.display()))?;
        let db = Arc::new(db);
        let clock = Arc::new(SystemClock::new(config.clock.time_zone()?));

        Ok(Self { library: Library::new(db.clone(), db, clock) })
    }
}
