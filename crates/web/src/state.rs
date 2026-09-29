use std::{ops::Deref, sync::Arc};

use dioxus::server::axum::{
    Extension,
    extract::{FromRequestParts, rejection::ExtensionRejection},
    http::request::Parts,
};
use yokoku_library::{Artworks, Calendar, Library};
use yokoku_media::Prober;

/// The use cases server functions call, wired by the composition root.
#[derive(Clone)]
pub struct AppState {
    pub library: Arc<Library>,
    pub artworks: Arc<Artworks>,
    pub calendar: Arc<Calendar>,
    pub prober: Arc<Prober>,
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

/// One dependency of a server function, taken from `AppState`: `library: Dep<Library>`.
pub struct Dep<T: ?Sized>(Arc<T>);

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
