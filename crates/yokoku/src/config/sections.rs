use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use yokoku_core::media::{RootFolder, RootKind};
use yokoku_domain::MonitorPreset;

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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct WebConfig {
    pub host: IpAddr,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self { host: Ipv4Addr::LOCALHOST.into(), port: 8080 }
    }
}

impl WebConfig {
    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EventsConfig {
    /// Milliseconds between checks for events another process stored, such as a setting changed
    /// from the command line.
    pub poll_interval_ms: u64,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self { poll_interval_ms: 5000 }
    }
}

/// A root folder from the config file.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RootConfig {
    pub kind: RootKind,
    pub path: PathBuf,
    /// Shown instead of the path; the folder's name when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl From<RootConfig> for RootFolder {
    fn from(root: RootConfig) -> Self {
        RootFolder::new(root.kind, root.path, root.name, true)
    }
}
