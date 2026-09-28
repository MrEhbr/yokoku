use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus::server::axum::Extension;

#[cfg(feature = "server")]
use crate::AppState;

#[get("/api/version", state: Extension<AppState>)]
pub async fn version() -> Result<String, ServerFnError> {
    Ok(state.version.to_owned())
}
