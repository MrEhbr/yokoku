use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yokoku_domain::MonitorPreset;
use yokoku_library::LibrarySort;

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
pub struct AddConfig {
    pub monitor: MonitorPreset,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct ListConfig {
    pub sort: LibrarySort,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct CalendarConfig {
    /// Days ahead to show instead of the week; the week while unset.
    pub days: Option<u16>,
}
