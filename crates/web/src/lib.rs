//! Yokoku's web UI: pages and server functions over the use cases, on Paper components restyled from Dioxus Components.

mod api;
pub mod components;
pub mod layout;
mod pages;
mod route;
#[cfg(feature = "server")]
mod server;

pub use route::App;
#[cfg(feature = "server")]
pub use server::{AppState, Server};
