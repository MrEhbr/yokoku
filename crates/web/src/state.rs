use std::{ops::Deref, sync::Arc};

use dioxus::server::axum::{
    Extension,
    extract::{FromRequestParts, rejection::ExtensionRejection},
    http::request::Parts,
};
use yokoku_domain::{Clock, Live, MonitorPreset};
use yokoku_downloads::Downloads;
use yokoku_events::{History, QueueChanges};
use yokoku_library::{Artworks, Calendar, Library, MetadataService};
use yokoku_media::{Deleter, Importer, Prober, Renamer, Reviewer, RootFolders};

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
    pub reviewer: Arc<Reviewer>,
    pub importer: Arc<Importer>,
    pub queue_changes: Arc<QueueChanges>,
    pub metadata: Arc<MetadataService>,
    pub roots: Arc<RootFolders>,
    pub add: Arc<AddSettings>,
    pub deleter: Arc<Deleter>,
    pub renamer: Arc<Renamer>,
}

/// The settings adding an item reads.
pub struct AddSettings {
    /// Searching and adding need a TMDB token.
    pub tmdb_token_set: Live<bool>,
    /// What a new series monitors; a new movie is monitored unless this is `None`.
    pub monitor: Live<MonitorPreset>,
}

/// `AppState` holds a `T`.
pub trait Provides<T: ?Sized> {
    fn provide(&self) -> Arc<T>;
}

impl Provides<Library> for AppState {
    fn provide(&self) -> Arc<Library> {
        self.library.clone()
    }
}

impl Provides<Artworks> for AppState {
    fn provide(&self) -> Arc<Artworks> {
        self.artworks.clone()
    }
}

impl Provides<Calendar> for AppState {
    fn provide(&self) -> Arc<Calendar> {
        self.calendar.clone()
    }
}

impl Provides<Prober> for AppState {
    fn provide(&self) -> Arc<Prober> {
        self.prober.clone()
    }
}

impl Provides<History> for AppState {
    fn provide(&self) -> Arc<History> {
        self.history.clone()
    }
}

impl Provides<dyn Clock> for AppState {
    fn provide(&self) -> Arc<dyn Clock> {
        self.clock.clone()
    }
}

impl Provides<Downloads> for AppState {
    fn provide(&self) -> Arc<Downloads> {
        self.downloads.clone()
    }
}

impl Provides<Reviewer> for AppState {
    fn provide(&self) -> Arc<Reviewer> {
        self.reviewer.clone()
    }
}

impl Provides<Importer> for AppState {
    fn provide(&self) -> Arc<Importer> {
        self.importer.clone()
    }
}

impl Provides<QueueChanges> for AppState {
    fn provide(&self) -> Arc<QueueChanges> {
        self.queue_changes.clone()
    }
}

impl Provides<MetadataService> for AppState {
    fn provide(&self) -> Arc<MetadataService> {
        self.metadata.clone()
    }
}

impl Provides<RootFolders> for AppState {
    fn provide(&self) -> Arc<RootFolders> {
        self.roots.clone()
    }
}

impl Provides<Deleter> for AppState {
    fn provide(&self) -> Arc<Deleter> {
        self.deleter.clone()
    }
}

impl Provides<Renamer> for AppState {
    fn provide(&self) -> Arc<Renamer> {
        self.renamer.clone()
    }
}

impl Provides<AddSettings> for AppState {
    fn provide(&self) -> Arc<AddSettings> {
        self.add.clone()
    }
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
