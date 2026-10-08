use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{Secret, TrackerId};
use yokoku_infra::{
    db::Database,
    indexers::{TorznabClient, TorznabFeed},
};
use yokoku_web::{FeedAccess, FeedDraft, FeedEntry};

use super::Settings;

pub struct FeedManager {
    settings: Settings,
    db: Arc<Database>,
}

impl FeedManager {
    pub fn new(settings: Settings, db: Arc<Database>) -> Self {
        Self { settings, db }
    }

    async fn checked(&self, draft: FeedDraft) -> Result<TorznabFeed, String> {
        let stored = self.db.torznab_feeds().await.map_err(|error| error.to_string())?;
        let id: TrackerId = match &draft.id {
            Some(id) => id.parse().map_err(|error: yokoku_domain::InvalidTrackerId| error.to_string())?,
            None => format!("feed_{}", uuid::Uuid::now_v7().simple()).parse().expect("generated feed id"),
        };
        if self.settings.current().torznab.feeds.iter().any(|feed| feed.id == id) {
            return Err("This feed is set in the config file; edit it there".into());
        }
        let old = stored.iter().find(|feed| feed.id == id);
        if draft.id.is_some() && old.is_none() {
            return Err("The feed no longer exists; reload Settings".into());
        }
        let api_key = match draft.key {
            None => old.and_then(|feed| feed.api_key.clone()),
            Some(key) if key.is_empty() => None,
            Some(key) => Some(Secret::new(key)),
        };
        let feed = TorznabFeed {
            id,
            name: draft.name.trim().to_owned(),
            url: draft.url.trim().to_owned(),
            api_key,
            enabled: draft.enabled,
        };
        feed.validate()?;
        Ok(feed)
    }
}

#[async_trait]
impl FeedAccess for FeedManager {
    async fn list(&self) -> Result<Vec<FeedEntry>, String> {
        let configured = self.settings.current().torznab.feeds.clone();
        let stored = self.db.torznab_feeds().await.map_err(|error| error.to_string())?;
        let mut entries: Vec<_> = configured
            .into_iter()
            .map(|feed| (feed, true))
            .chain(stored.into_iter().map(|feed| (feed, false)))
            .map(|(feed, configured)| FeedEntry {
                id: feed.id.to_string(),
                name: feed.name,
                url: feed.url,
                key_set: feed.api_key.is_some(),
                enabled: feed.enabled,
                configured,
            })
            .collect();
        entries.sort_by_key(|entry| entry.name.to_lowercase());
        Ok(entries)
    }

    async fn save(&self, draft: FeedDraft) -> Result<(), String> {
        let feed = self.checked(draft).await?;
        self.db.save_torznab_feed(&feed).await.map_err(|error| error.to_string())
    }

    async fn remove(&self, id: String) -> Result<(), String> {
        if self.settings.current().torznab.feeds.iter().any(|feed| feed.id.as_str() == id) {
            return Err("This feed is set in the config file; edit it there".into());
        }
        if !self.db.remove_torznab_feed(&id).await.map_err(|error| error.to_string())? {
            return Err("The feed no longer exists; reload Settings".into());
        }
        Ok(())
    }

    async fn test(&self, draft: FeedDraft) -> Result<String, String> {
        if let Some(feed) = draft
            .id
            .as_ref()
            .and_then(|id| self.settings.current().torznab.feeds.iter().find(|feed| feed.id.as_str() == id).cloned())
        {
            return TorznabClient::new().test(&feed).await.map_err(|error| error.to_string());
        }
        let feed = self.checked(draft).await?;
        TorznabClient::new().test(&feed).await.map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn editing_a_feed_preserves_or_clears_its_key_explicitly() {
        let db = Arc::new(Database::open_in_memory().await.unwrap());
        let manager = FeedManager::new(Settings::open(None, db.clone()).await.unwrap(), db.clone());
        let mut draft = FeedDraft {
            id: None,
            name: "Example".into(),
            url: "https://indexer.example/api".into(),
            key: Some("private-key".into()),
            enabled: true,
        };
        manager.save(draft.clone()).await.unwrap();
        draft.id = Some(manager.list().await.unwrap()[0].id.clone());

        draft.key = None;
        manager.save(draft.clone()).await.unwrap();
        assert_eq!(db.torznab_feeds().await.unwrap()[0].api_key.as_ref().map(Secret::expose), Some("private-key"));

        draft.key = Some(String::new());
        manager.save(draft.clone()).await.unwrap();
        assert!(db.torznab_feeds().await.unwrap()[0].api_key.is_none());

        manager.remove(draft.id.unwrap()).await.unwrap();
        assert!(manager.list().await.unwrap().is_empty());
    }
}
