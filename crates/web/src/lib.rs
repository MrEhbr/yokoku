//! Yokoku's web UI: pages and server functions over the use cases, on Paper components restyled from Dioxus Components.

mod api;
pub mod components;
mod dialogs;
mod format;
pub mod layout;
mod pages;
mod route;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
mod state;

pub use route::App;
#[cfg(feature = "server")]
pub use {
    server::Server,
    state::{AddSettings, AppState},
};
