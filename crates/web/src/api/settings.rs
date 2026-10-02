//! The settings in effect, the root folders and the library scan, as the Settings page edits them.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::library::Kind;
#[cfg(feature = "server")]
use crate::{
    api::{Dep, RootFolders, Scanner},
    state::{ConnectionTest, SettingsAccess},
};

/// Where a setting sits on the Settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Section {
    DownloadClient,
    MediaServer,
    Metadata,
    Library,
    Import,
    Naming,
    Files,
    Schedules,
    Server,
}

/// How a setting is edited.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Control {
    /// With its placeholder.
    Text(String),
    /// Shown masked; typing replaces it.
    Secret,
    /// JSON string values with their labels; a value in effect outside them is offered too.
    Choice(Vec<(String, String)>),
    Switch,
    /// Strings, typed comma-separated.
    List,
    /// Shown, not edited: read when the service starts, from the config file or environment.
    ReadOnly,
}

#[cfg(any(feature = "server", test))]
impl Control {
    pub fn text(placeholder: &str) -> Self {
        Self::Text(placeholder.to_owned())
    }

    pub fn choice(choices: &[(&str, &str)]) -> Self {
        Self::Choice(choices.iter().map(|(value, label)| ((*value).to_owned(), (*label).to_owned())).collect())
    }
}

/// A setting the Settings page edits, as the configuration describes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Dotted, like `import.mode`.
    pub key: String,
    pub section: Section,
    pub label: String,
    pub hint: String,
    pub control: Control,
}

#[cfg(any(feature = "server", test))]
impl Field {
    pub fn new(section: Section, key: &str, label: &str, hint: &str, control: Control) -> Self {
        Self { key: key.to_owned(), section, label: label.to_owned(), hint: hint.to_owned(), control }
    }
}

/// A setting's value in effect, as JSON, with a secret masked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    pub field: Field,
    pub value: Value,
    /// Stored in the database, over the config file.
    pub stored: bool,
    /// Set by a `YOKOKU__` environment variable, which the page cannot change.
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
#[post("/api/settings/test", access: Dep<dyn SettingsAccess>, connections: Dep<dyn ConnectionTest>)]
pub async fn test_connection(connection: Connection, changes: Vec<(String, Value)>) -> Result<String, ServerFnError> {
    server::test(&*access, &*connections, connection, changes).await
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

/// Links new files in the item folders and forgets those gone from disk.
#[post("/api/library/scan", scanner: Dep<Scanner>)]
pub async fn scan_library() -> Result<Scanned, ServerFnError> {
    server::scan(&scanner).await
}

#[cfg(feature = "server")]
mod server {
    use std::path::Path;

    use dioxus::{logger::tracing::error, prelude::*};
    use serde_json::Value;
    use yokoku_core::media::MediaError;

    use super::{
        Connection, ConnectionTest, Control, Field, Kind, Root, RootFolders, Scanned, Scanner, Setting, SettingsAccess,
    };
    use crate::api::{root_listing_failed, unexpected};

    pub(super) async fn settings(access: &dyn SettingsAccess) -> Result<Vec<Setting>, ServerFnError> {
        let stored = access.stored_keys().await.map_err(|error| {
            error!(%error, "reading the stored settings failed");
            ServerFnError::new("The settings could not be loaded")
        })?;
        Ok(access.fields().into_iter().filter_map(|field| setting(access, &stored, field)).collect())
    }

    pub(super) async fn save(access: &dyn SettingsAccess, key: &str, value: Value) -> Result<Setting, ServerFnError> {
        let field = editable(access, key)?;
        let saved = if cleared(&value) { access.unset(key).await } else { access.set(key, value).await };
        saved.map_err(ServerFnError::new)?;
        let stored = access.stored_keys().await.map_err(ServerFnError::new)?;
        setting(access, &stored, field).ok_or_else(|| ServerFnError::new(format!("{key} is not a setting")))
    }

    pub(super) async fn test(
        access: &dyn SettingsAccess,
        connections: &dyn ConnectionTest,
        connection: Connection,
        changes: Vec<(String, Value)>,
    ) -> Result<String, ServerFnError> {
        for (key, _) in &changes {
            editable(access, key)?;
        }
        let changes =
            changes.into_iter().map(|(key, value)| (key, Some(value).filter(|value| !cleared(value)))).collect();
        connections.test(connection, changes).await.map_err(ServerFnError::new)
    }

    /// Null, blank text or an empty list; such a value unsets the key.
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

    /// The page's editable field for `key`; no other key can be changed through it.
    fn editable(access: &dyn SettingsAccess, key: &str) -> Result<Field, ServerFnError> {
        access
            .fields()
            .into_iter()
            .find(|field| field.key == key && field.control != Control::ReadOnly)
            .ok_or_else(|| ServerFnError::new(format!("{key} cannot be changed here")))
    }

    fn setting(access: &dyn SettingsAccess, stored: &[String], field: Field) -> Option<Setting> {
        Some(Setting {
            value: access.value(&field.key)?,
            stored: stored.contains(&field.key),
            from_env: access.set_by_env(&field.key),
            field,
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
