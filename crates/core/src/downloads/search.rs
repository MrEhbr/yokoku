use std::{
    cmp::Reverse,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tracing::{info, instrument};
use yokoku_domain::ItemId;

use crate::downloads::{
    Download, DownloadError, Downloads,
    ports::{Indexer, ReleaseQuery, SearchResult, Tracker},
};

/// Finds releases on the indexer and adds the chosen one as a download.
pub struct ReleaseSearch {
    indexer: Arc<dyn Indexer>,
    downloads: Arc<Downloads>,
    links: Mutex<HashMap<String, (String, Instant)>>,
}

const LINK_LIFETIME: Duration = Duration::from_secs(30 * 60);
const MAX_LINKS: usize = 10_000;

impl ReleaseSearch {
    pub fn new(indexer: Arc<dyn Indexer>, downloads: Arc<Downloads>) -> Self {
        Self { indexer, downloads, links: Mutex::new(HashMap::new()) }
    }

    /// The trackers a search can be narrowed to.
    pub async fn trackers(&self) -> Result<Vec<Tracker>, DownloadError> {
        Ok(self.indexer.trackers().await?)
    }

    /// Most seeded first.
    #[instrument(skip_all, fields(text = %query.text, kind = ?query.kind, season = ?query.season, episode = ?query.episode, trackers = ?query.trackers))]
    pub async fn search(&self, query: &ReleaseQuery) -> Result<SearchResult, DownloadError> {
        let mut result = self.indexer.search(query).await?;
        result.releases.sort_by_key(|release| Reverse(release.seeders));
        let now = Instant::now();
        let mut links = self.links.lock().expect("release links lock");
        links.retain(|_, (_, created)| now.duration_since(*created) < LINK_LIFETIME);
        for release in &mut result.releases {
            let id = uuid::Uuid::now_v7().to_string();
            links.insert(id.clone(), (std::mem::take(&mut release.link), now));
            release.link = id;
        }
        while links.len() > MAX_LINKS {
            let oldest = links.iter().min_by_key(|(_, (_, created))| created).map(|(id, _)| id.clone());
            if let Some(oldest) = oldest {
                links.remove(&oldest);
            }
        }
        info!(found = result.releases.len(), warnings = result.warnings.len(), "releases searched");
        Ok(result)
    }

    /// Adds a recent search result as `Downloads::add` does; raw download links stay on the server.
    pub async fn grab(&self, id: &str, item: Option<ItemId>, season: Option<u16>) -> Result<Download, DownloadError> {
        let link = self
            .links
            .lock()
            .expect("release links lock")
            .get(id)
            .and_then(|(link, created)| (created.elapsed() < LINK_LIFETIME).then(|| link.clone()))
            .ok_or_else(|| crate::downloads::ports::IndexerError::Refused("Search expired; search again".into()))?;
        let torrent = self.indexer.fetch(&link).await?;
        self.downloads.add(&torrent, item, season).await
    }
}
