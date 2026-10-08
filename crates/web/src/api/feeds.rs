use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::{api::Dep, state::FeedAccess};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedEntry {
    pub id: String,
    pub name: String,
    pub url: String,
    pub key_set: bool,
    pub enabled: bool,
    pub configured: bool,
}

/// `key` absent keeps the old key when editing; an empty string clears it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedDraft {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
    pub key: Option<String>,
    pub enabled: bool,
}

#[get("/api/indexers/feeds", access: Dep<dyn FeedAccess>)]
pub async fn feeds() -> Result<Vec<FeedEntry>, ServerFnError> {
    access.list().await.map_err(ServerFnError::new)
}

#[post("/api/indexers/feeds", access: Dep<dyn FeedAccess>)]
pub async fn save_feed(draft: FeedDraft) -> Result<(), ServerFnError> {
    access.save(draft).await.map_err(ServerFnError::new)
}

#[post("/api/indexers/feeds/remove", access: Dep<dyn FeedAccess>)]
pub async fn remove_feed(id: String) -> Result<(), ServerFnError> {
    access.remove(id).await.map_err(ServerFnError::new)
}

#[post("/api/indexers/feeds/test", access: Dep<dyn FeedAccess>)]
pub async fn test_feed(draft: FeedDraft) -> Result<String, ServerFnError> {
    access.test(draft).await.map_err(ServerFnError::new)
}
