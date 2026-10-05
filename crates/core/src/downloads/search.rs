use std::{cmp::Reverse, sync::Arc};

use tracing::{info, instrument};
use yokoku_domain::ItemId;

use crate::downloads::{
    Download, DownloadError, Downloads,
    ports::{Indexer, Release, ReleaseQuery},
};

/// Finds releases on the indexer and adds the chosen one as a download.
pub struct ReleaseSearch {
    indexer: Arc<dyn Indexer>,
    downloads: Arc<Downloads>,
}

impl ReleaseSearch {
    pub fn new(indexer: Arc<dyn Indexer>, downloads: Arc<Downloads>) -> Self {
        Self { indexer, downloads }
    }

    /// Most seeded first.
    #[instrument(skip_all, fields(text = %query.text, kind = ?query.kind, season = ?query.season, episode = ?query.episode))]
    pub async fn search(&self, query: &ReleaseQuery) -> Result<Vec<Release>, DownloadError> {
        let mut releases = self.indexer.search(query).await?;
        releases.sort_by_key(|release| Reverse(release.seeders));
        info!(found = releases.len(), "releases searched");
        Ok(releases)
    }

    /// Adds the release at `link` as `Downloads::add` does.
    pub async fn grab(&self, link: &str, item: Option<ItemId>, season: Option<u16>) -> Result<Download, DownloadError> {
        let torrent = self.indexer.fetch(link).await?;
        self.downloads.add(&torrent, item, season).await
    }
}
