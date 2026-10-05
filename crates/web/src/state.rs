use std::{ops::Deref, sync::Arc};

use dioxus::server::axum::{
    Extension,
    extract::{FromRequestParts, rejection::ExtensionRejection},
    http::request::Parts,
};
use tokio_util::sync::CancellationToken;
use yokoku_core::{
    downloads::{Downloads, ReleaseSearch},
    events::{History, QueueChanges},
    library::{Artworks, Calendar, Library, MetadataService},
    media::{Deleter, Importer, Prober, Renamer, Reviewer, RootFolders, Scanner},
};
use yokoku_domain::{Clock, Live, MonitorPreset};

/// The use cases server functions call, wired by the composition root.
#[derive(Clone)]
pub struct AppState {
    pub library: Arc<Library>,
    pub artworks: Arc<Artworks>,
    pub calendar: Arc<Calendar>,
    pub prober: Arc<Prober>,
    pub history: Arc<History>,
    pub clock: Arc<dyn Clock>,
    pub downloads: Arc<Downloads>,
    pub releases: Arc<ReleaseSearch>,
    pub reviewer: Arc<Reviewer>,
    pub importer: Arc<Importer>,
    pub queue_changes: Arc<QueueChanges>,
    pub metadata: Arc<MetadataService>,
    pub roots: Arc<RootFolders>,
    pub add: Arc<AddSettings>,
    pub deleter: Arc<Deleter>,
    pub renamer: Arc<Renamer>,
    pub scanner: Arc<Scanner>,
    pub settings: Arc<dyn SettingsAccess>,
    pub connections: Arc<dyn ConnectionTest>,
    /// Cancelled when the service stops; long-lived responses end with it.
    pub shutdown: Arc<CancellationToken>,
}

/// The settings adding an item reads.
pub struct AddSettings {
    /// Searching and adding need a TMDB token.
    pub tmdb_token_set: Live<bool>,
    /// What a new series monitors; a new movie is monitored unless this is `None`.
    pub monitor: Live<MonitorPreset>,
}

/// The configuration as the Settings page reads and changes it.
#[async_trait::async_trait]
pub trait SettingsAccess: Send + Sync {
    /// The settings the Settings page shows, in page order.
    fn fields(&self) -> Vec<crate::api::settings::Field>;

    /// The value in effect as JSON, a secret masked; `None` for a key that is not a setting.
    fn value(&self, key: &str) -> Option<serde_json::Value>;

    /// A `YOKOKU__` environment variable sets `key`, over any stored value.
    fn set_by_env(&self, key: &str) -> bool;

    async fn stored_keys(&self) -> Result<Vec<String>, String>;

    /// Stores and applies `value`; the error says why it does not load.
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<(), String>;

    /// Removes the stored value, so the config file or the default applies again.
    async fn unset(&self, key: &str) -> Result<(), String>;
}

/// Reaches a service with settings that are not stored yet.
#[async_trait::async_trait]
pub trait ConnectionTest: Send + Sync {
    /// The service's version, or why it could not be reached, with `changes` over the settings
    /// in effect and nothing stored; a `None` value leaves its key to the config file.
    async fn test(
        &self,
        connection: crate::api::settings::Connection,
        changes: Vec<(String, Option<serde_json::Value>)>,
    ) -> Result<String, String>;
}

/// `AppState` holds a `T`.
pub trait Provides<T: ?Sized> {
    fn provide(&self) -> Arc<T>;
}

macro_rules! provides {
    ($($ty:ty => $field:ident,)*) => {
        $(impl Provides<$ty> for AppState {
            fn provide(&self) -> Arc<$ty> {
                self.$field.clone()
            }
        })*
    };
}

provides! {
    Library => library,
    Artworks => artworks,
    Calendar => calendar,
    Prober => prober,
    History => history,
    dyn Clock => clock,
    Downloads => downloads,
    ReleaseSearch => releases,
    Reviewer => reviewer,
    Importer => importer,
    QueueChanges => queue_changes,
    MetadataService => metadata,
    RootFolders => roots,
    Deleter => deleter,
    Renamer => renamer,
    Scanner => scanner,
    dyn SettingsAccess => settings,
    dyn ConnectionTest => connections,
    AddSettings => add,
    CancellationToken => shutdown,
}

/// One dependency of a server function, taken from `AppState`: `library: Dep<Library>`.
pub struct Dep<T: ?Sized>(Arc<T>);

impl<T: ?Sized> Dep<T> {
    /// For work that outlives the request, like a stream.
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T: ?Sized> Deref for Dep<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<S: Send + Sync, T: ?Sized> FromRequestParts<S> for Dep<T>
where
    AppState: Provides<T>,
{
    type Rejection = ExtensionRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(app) = Extension::<AppState>::from_request_parts(parts, state).await?;
        Ok(Self(app.provide()))
    }
}
