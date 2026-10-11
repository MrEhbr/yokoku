use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use yokoku_core::downloads::ports::{Indexer, IndexerError, ReleaseQuery, SearchResult, TorrentSource, Tracker};
use yokoku_domain::{Live, TrackerId, TrackerSet, Trackers};

use super::{JackettClient, JackettSettings, ProwlarrClient, ProwlarrSettings};

/// The proxy used for release searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexerProvider {
    Jackett,
    Prowlarr,
}

/// Routes searches to the selected proxy and fetches results through their original proxy.
pub struct SelectedIndexer {
    jackett: JackettClient,
    prowlarr: ProwlarrClient,
    provider: Live<IndexerProvider>,
}

impl SelectedIndexer {
    pub fn new(
        jackett: Live<JackettSettings>,
        prowlarr: Live<ProwlarrSettings>,
        provider: Live<IndexerProvider>,
    ) -> Self {
        Self { jackett: JackettClient::new(jackett), prowlarr: ProwlarrClient::new(prowlarr), provider }
    }
}

#[async_trait]
impl Indexer for SelectedIndexer {
    async fn version(&self) -> Result<String, IndexerError> {
        match self.provider.current() {
            IndexerProvider::Jackett => self.jackett.version().await,
            IndexerProvider::Prowlarr => self.prowlarr.version().await,
        }
    }

    async fn trackers(&self) -> Result<Vec<Tracker>, IndexerError> {
        let (trackers, prefix, name) = match self.provider.current() {
            IndexerProvider::Jackett => (self.jackett.trackers().await?, "jackett_", "Jackett"),
            IndexerProvider::Prowlarr => (self.prowlarr.trackers().await?, "prowlarr_", "Prowlarr"),
        };
        Ok(trackers
            .into_iter()
            .map(|tracker| Tracker {
                id: format!("{prefix}{}", tracker.id).parse().expect("tracker id is valid"),
                name: format!("{} · {name}", tracker.name),
            })
            .collect())
    }

    async fn search(&self, query: &ReleaseQuery) -> Result<SearchResult, IndexerError> {
        let (mut found, prefix, name) = match self.provider.current() {
            IndexerProvider::Jackett => {
                (self.jackett.search(&selected_query(query, "jackett_")?).await?, "j:", "Jackett")
            },
            IndexerProvider::Prowlarr => {
                (self.prowlarr.search(&selected_query(query, "prowlarr_")?).await?, "p:", "Prowlarr")
            },
        };
        for release in &mut found.releases {
            release.tracker =
                if release.tracker.is_empty() { name.to_owned() } else { format!("{} · {name}", release.tracker) };
            release.link = format!("{prefix}{}", release.link);
        }
        Ok(found)
    }

    async fn fetch(&self, link: &str) -> Result<TorrentSource, IndexerError> {
        if let Some(link) = link.strip_prefix("j:") {
            self.jackett.fetch(link).await
        } else if let Some(link) = link.strip_prefix("p:") {
            self.prowlarr.fetch(link).await
        } else {
            Err(IndexerError::Refused("Search expired; search again".into()))
        }
    }
}

fn selected_query(query: &ReleaseQuery, prefix: &str) -> Result<ReleaseQuery, IndexerError> {
    let mut selected = query.clone();
    if let Trackers::Only(set) = &query.trackers {
        let ids: Vec<TrackerId> = set
            .ids()
            .iter()
            .map(|id| {
                id.as_str()
                    .strip_prefix(prefix)
                    .ok_or_else(|| IndexerError::Refused("Choose a tracker from the selected indexer".into()))?
                    .parse()
                    .map_err(|_| IndexerError::Refused("Choose a valid tracker".into()))
            })
            .collect::<Result<_, _>>()?;
        selected.trackers = Trackers::Only(TrackerSet::new(ids).expect("selected trackers are nonempty"));
    }
    Ok(selected)
}
