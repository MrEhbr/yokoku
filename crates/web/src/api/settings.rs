//! The settings in effect, the root folders and the library scan, as the Settings page edits them
//! (FR-10.1, 3.1, 8.1, 8.2).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::library::Kind;
#[cfg(feature = "server")]
use crate::{
    api::{Dep, RootFolders, Scanner},
    state::SettingsAccess,
};

/// A setting's value in effect, as JSON, with a secret masked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: Value,
    /// Stored in the database, over the config file.
    pub stored: bool,
    /// Set by an `APP__` environment variable, which the page cannot change.
    pub from_env: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Connection {
    Transmission,
    Jellyfin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub kind: Kind,
    pub path: String,
    /// Library items in it.
    pub items: usize,
}

#[get("/api/settings", access: Dep<dyn SettingsAccess>)]
pub async fn settings() -> Result<Vec<Setting>, ServerFnError> {
    server::settings(&*access).await
}

/// Stores `value` for `key` over the config file; an empty text or list removes the stored value.
#[post("/api/settings", access: Dep<dyn SettingsAccess>)]
pub async fn save_setting(key: String, value: Value) -> Result<Setting, ServerFnError> {
    server::save(&*access, &key, value).await
}

/// Removes the stored value of `key`, so the config file or the default applies again.
#[post("/api/settings/reset", access: Dep<dyn SettingsAccess>)]
pub async fn reset_setting(key: String) -> Result<Setting, ServerFnError> {
    server::save(&*access, &key, Value::Null).await
}

/// What the service answered, such as its version, with `changes` (unsaved values, as for
/// `save_setting`) over the settings in effect.
#[post("/api/settings/test", access: Dep<dyn SettingsAccess>)]
pub async fn test_connection(connection: Connection, changes: Vec<(String, Value)>) -> Result<String, ServerFnError> {
    server::test(&*access, connection, changes).await
}

#[get("/api/roots", roots: Dep<RootFolders>)]
pub async fn roots() -> Result<Vec<Root>, ServerFnError> {
    server::roots(&roots).await
}

#[post("/api/roots", roots: Dep<RootFolders>)]
pub async fn add_root(kind: Kind, path: String) -> Result<(), ServerFnError> {
    server::add_root(&roots, kind, &path).await
}

/// Refused while library items belong to it.
#[post("/api/roots/remove", roots: Dep<RootFolders>)]
pub async fn remove_root(path: String) -> Result<(), ServerFnError> {
    server::remove_root(&roots, &path).await
}

/// What a library scan changed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scanned {
    /// New files linked to their items.
    pub found: usize,
    /// Linked files no longer on disk.
    pub vanished: usize,
    /// Folders with files to match by hand.
    pub unrecognised: usize,
}

/// Links new files in the item folders and forgets those gone from disk (FR-8.2, 8.7).
#[post("/api/library/scan", scanner: Dep<Scanner>)]
pub async fn scan_library() -> Result<Scanned, ServerFnError> {
    server::scan(&scanner).await
}

#[cfg(feature = "server")]
mod server {
    use std::path::Path;

    use dioxus::{logger::tracing::error, prelude::*};
    use serde_json::Value;
    use yokoku_media::MediaError;

    use super::{Connection, Kind, Root, RootFolders, Scanned, Scanner, Setting, SettingsAccess};
    use crate::api::{root_listing_failed, unexpected};

    /// The settings the page shows; no other key can be changed through it.
    const KEYS: &[&str] = &[
        "transmission.url",
        "transmission.username",
        "transmission.password",
        "jellyfin.url",
        "jellyfin.api_key",
        "metadata.tmdb.token",
        "metadata.tvdb.api_key",
        "metadata.tvdb.pin",
        "metadata.language",
        "metadata.region",
        "add.monitor",
        "clock.timezone",
        "import.mode",
        "downloads.remove_after_seeding",
        "downloads.pick_up_labels",
        "downloads.pick_up_folder",
        "naming.movie_folder",
        "naming.movie_file",
        "naming.series_folder",
        "naming.season_folder",
        "naming.episode_file",
        "files.ffprobe",
    ];

    pub(super) async fn settings(access: &dyn SettingsAccess) -> Result<Vec<Setting>, ServerFnError> {
        let stored = access.stored_keys().await.map_err(|error| {
            error!(%error, "reading the stored settings failed");
            ServerFnError::new("The settings could not be loaded")
        })?;
        Ok(KEYS.iter().filter_map(|key| setting(access, &stored, key)).collect())
    }

    pub(super) async fn save(access: &dyn SettingsAccess, key: &str, value: Value) -> Result<Setting, ServerFnError> {
        if !KEYS.contains(&key) {
            return Err(ServerFnError::new(format!("{key} cannot be changed here")));
        }
        let saved = if cleared(&value) { access.unset(key).await } else { access.set(key, value).await };
        saved.map_err(ServerFnError::new)?;
        let stored = access.stored_keys().await.map_err(ServerFnError::new)?;
        setting(access, &stored, key).ok_or_else(|| ServerFnError::new(format!("{key} is not a setting")))
    }

    pub(super) async fn test(
        access: &dyn SettingsAccess,
        connection: Connection,
        changes: Vec<(String, Value)>,
    ) -> Result<String, ServerFnError> {
        if let Some((key, _)) = changes.iter().find(|(key, _)| !KEYS.contains(&key.as_str())) {
            return Err(ServerFnError::new(format!("{key} cannot be changed here")));
        }
        let changes =
            changes.into_iter().map(|(key, value)| (key, Some(value).filter(|value| !cleared(value)))).collect();
        access.test(connection, changes).await.map_err(ServerFnError::new)
    }

    /// Empty, so the config file's value applies.
    fn cleared(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(text) => text.trim().is_empty(),
            Value::Array(items) => items.is_empty(),
            _ => false,
        }
    }

    pub(super) async fn roots(roots: &RootFolders) -> Result<Vec<Root>, ServerFnError> {
        let mut listed = Vec::new();
        for root in roots.list().await.map_err(root_listing_failed)? {
            let items = roots.item_folders(&root).await.map_err(root_listing_failed)?.len();
            listed.push(Root { kind: root.kind.into(), path: root.path.display().to_string(), items });
        }
        Ok(listed)
    }

    pub(super) async fn add_root(roots: &RootFolders, kind: Kind, path: &str) -> Result<(), ServerFnError> {
        roots.add(kind.into(), Path::new(path.trim())).await.map(drop).map_err(root_failure)
    }

    pub(super) async fn remove_root(roots: &RootFolders, path: &str) -> Result<(), ServerFnError> {
        roots.remove(Path::new(path)).await.map_err(root_failure)
    }

    pub(super) async fn scan(scanner: &Scanner) -> Result<Scanned, ServerFnError> {
        let report = scanner.scan().await.map_err(|error| {
            error!(%error, "scanning the library failed");
            ServerFnError::new("The library could not be scanned; the server log has the cause")
        })?;
        Ok(Scanned { found: report.found, vanished: report.vanished, unrecognised: report.needs_review.len() })
    }

    fn setting(access: &dyn SettingsAccess, stored: &[String], key: &str) -> Option<Setting> {
        Some(Setting {
            value: access.value(key)?,
            stored: stored.iter().any(|stored| stored == key),
            from_env: access.set_by_env(key),
            key: key.to_owned(),
        })
    }

    fn root_failure(error: MediaError) -> ServerFnError {
        match error {
            MediaError::RelativePath(_)
            | MediaError::NotAFolder(_)
            | MediaError::OverlappingRoot { .. }
            | MediaError::RootNotFound(_)
            | MediaError::RootInUse { .. } => ServerFnError::new(error.to_string()),
            error => unexpected(&error, "changing root folders"),
        }
    }
}
