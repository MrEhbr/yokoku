use std::sync::Arc;

use async_trait::async_trait;
use futures_util::future::join_all;
use tracing::warn;
use yokoku_core::downloads::ports::{Indexer, IndexerError, ReleaseQuery, SearchResult, TorrentSource, Tracker};
use yokoku_domain::{Live, TrackerId, TrackerSet, Trackers};

use crate::{
    db::Database,
    indexers::{JackettClient, JackettSettings, TorznabClient, TorznabFeed, TorznabSettings},
};

/// Jackett's configured trackers and direct Torznab feeds as one search source.
pub struct CombinedIndexer {
    jackett: JackettClient,
    jackett_settings: Live<JackettSettings>,
    direct: TorznabClient,
    feeds: Live<TorznabSettings>,
    db: Arc<Database>,
}

impl CombinedIndexer {
    pub fn new(jackett: Live<JackettSettings>, feeds: Live<TorznabSettings>, db: Arc<Database>) -> Self {
        Self {
            jackett: JackettClient::new(jackett.clone()),
            jackett_settings: jackett,
            direct: TorznabClient::new(),
            feeds,
            db,
        }
    }

    pub async fn feeds(&self) -> Result<Vec<(TorznabFeed, bool)>, IndexerError> {
        let mut feeds: Vec<_> = self.feeds.current().feeds.into_iter().map(|feed| (feed, true)).collect();
        let stored = self.db.torznab_feeds().await.map_err(|error| IndexerError::Unavailable(error.into()))?;
        for feed in stored {
            if feeds.iter().any(|(current, _)| current.id == feed.id) {
                return Err(IndexerError::Refused(format!("duplicate Torznab feed id: {}", feed.id)));
            }
            feeds.push((feed, false));
        }
        Ok(feeds)
    }
}

#[async_trait]
impl Indexer for CombinedIndexer {
    async fn version(&self) -> Result<String, IndexerError> {
        if self.jackett_settings.current().url.is_some() {
            return self.jackett.version().await;
        }
        let feed = self.feeds().await?.into_iter().find(|(feed, _)| feed.enabled).ok_or(IndexerError::NotConfigured)?.0;
        self.direct.test(&feed).await
    }

    async fn trackers(&self) -> Result<Vec<Tracker>, IndexerError> {
        let feeds = self.feeds().await?;
        let mut listed: Vec<Tracker> = feeds
            .into_iter()
            .filter(|(feed, _)| feed.enabled)
            .map(|(feed, _)| Tracker {
                id: format!("torznab_{}", feed.id).parse().expect("feed id is valid"),
                name: feed.name,
            })
            .collect();
        if self.jackett_settings.current().url.is_some() {
            match self.jackett.trackers().await {
                Ok(trackers) => listed.extend(trackers.into_iter().map(|tracker| Tracker {
                    id: format!("jackett_{}", tracker.id).parse().expect("tracker id is valid"),
                    name: format!("{} · Jackett", tracker.name),
                })),
                Err(error) if !listed.is_empty() => warn!(%error, "Jackett trackers could not be listed"),
                Err(error) => return Err(error),
            }
        }
        if listed.is_empty() {
            return Err(IndexerError::NotConfigured);
        }
        Ok(listed)
    }

    async fn search(&self, query: &ReleaseQuery) -> Result<SearchResult, IndexerError> {
        let all_feeds = self.feeds().await?;
        let chosen = match &query.trackers {
            Trackers::All => None,
            Trackers::Only(set) => Some(set.ids()),
        };
        let feeds: Vec<_> = all_feeds
            .into_iter()
            .filter(|(feed, _)| feed.enabled)
            .filter(|(feed, _)| {
                chosen.is_none_or(|ids| ids.iter().any(|id| id.as_str() == format!("torznab_{}", feed.id)))
            })
            .map(|(feed, _)| feed)
            .collect();
        let jackett = self.jackett_settings.current().url.is_some()
            && chosen.is_none_or(|ids| ids.iter().any(|id| id.as_str().starts_with("jackett_")));
        let jackett_query = if let Some(ids) = chosen {
            let selected = ids.iter().filter_map(|id| {
                id.as_str().strip_prefix("jackett_").and_then(|plain| plain.parse::<TrackerId>().ok())
            });
            let mut copy = query.clone();
            if jackett {
                copy.trackers = Trackers::Only(
                    TrackerSet::new(selected)
                        .ok_or_else(|| IndexerError::Refused("Choose a valid Jackett tracker".into()))?,
                );
            }
            copy
        } else {
            query.clone()
        };
        let jackett_search = async { if jackett { Some(self.jackett.search(&jackett_query).await) } else { None } };
        let direct_search = join_all(feeds.iter().map(|feed| self.direct.search(feed, query)));
        let (from_jackett, from_direct) = tokio::join!(jackett_search, direct_search);
        let mut releases = Vec::new();
        let mut warnings = Vec::new();
        let mut success = false;
        let mut last_error = None;
        if let Some(result) = from_jackett {
            match result {
                Ok(found) => {
                    success = true;
                    warnings.extend(found.warnings);
                    releases.extend(found.releases.into_iter().map(|mut release| {
                        release.link = format!("j:{}", release.link);
                        release
                    }));
                },
                Err(error) => {
                    warn!(%error, "Jackett search failed");
                    warnings.push("Jackett could not be searched".into());
                    last_error = Some(error);
                },
            }
        }
        for (feed, result) in feeds.iter().zip(from_direct) {
            match result {
                Ok(found) => {
                    success = true;
                    releases.extend(found.into_iter().map(|mut release| {
                        release.link = format!("d:{}:{}", feed.id, release.link);
                        release
                    }));
                },
                Err(error) => {
                    warn!(feed = %feed.name, %error, "Torznab search failed");
                    warnings.push(format!("{} could not be searched", feed.name));
                    last_error = Some(error);
                },
            }
        }
        if success {
            Ok(SearchResult { releases, warnings })
        } else {
            Err(last_error.unwrap_or(IndexerError::NotConfigured))
        }
    }

    async fn fetch(&self, link: &str) -> Result<TorrentSource, IndexerError> {
        if let Some(link) = link.strip_prefix("j:") {
            return self.jackett.fetch(link).await;
        }
        let Some(rest) = link.strip_prefix("d:") else {
            return Err(IndexerError::Refused("Search expired; search again".into()));
        };
        let Some((id, link)) = rest.split_once(':') else {
            return Err(IndexerError::Refused("Search expired; search again".into()));
        };
        let available = self.feeds().await?.into_iter().any(|(feed, _)| feed.enabled && feed.id.as_str() == id);
        if !available {
            return Err(IndexerError::Refused("The feed is no longer available; search again".into()));
        }
        self.direct.fetch(link).await
    }
}
