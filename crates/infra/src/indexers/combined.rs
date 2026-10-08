use async_trait::async_trait;
use tracing::warn;
use yokoku_core::downloads::ports::{Indexer, IndexerError, ReleaseQuery, SearchResult, TorrentSource, Tracker};
use yokoku_domain::{Live, TrackerId, TrackerSet, Trackers};

use super::{JackettClient, JackettSettings, ProwlarrClient, ProwlarrSettings};

/// The trackers configured in Jackett and Prowlarr as one search source.
pub struct CombinedIndexer {
    jackett: JackettClient,
    jackett_settings: Live<JackettSettings>,
    prowlarr: ProwlarrClient,
    prowlarr_settings: Live<ProwlarrSettings>,
}

impl CombinedIndexer {
    pub fn new(jackett: Live<JackettSettings>, prowlarr: Live<ProwlarrSettings>) -> Self {
        Self {
            jackett: JackettClient::new(jackett.clone()),
            jackett_settings: jackett,
            prowlarr: ProwlarrClient::new(prowlarr.clone()),
            prowlarr_settings: prowlarr,
        }
    }
}

#[async_trait]
impl Indexer for CombinedIndexer {
    async fn version(&self) -> Result<String, IndexerError> {
        if self.jackett_settings.current().url.is_some() {
            self.jackett.version().await
        } else {
            self.prowlarr.version().await
        }
    }

    async fn trackers(&self) -> Result<Vec<Tracker>, IndexerError> {
        let mut listed = Vec::new();
        let mut last_error = None;
        let (jackett, prowlarr) = tokio::join!(
            async {
                if self.jackett_settings.current().url.is_some() { Some(self.jackett.trackers().await) } else { None }
            },
            async {
                if self.prowlarr_settings.current().url.is_some() { Some(self.prowlarr.trackers().await) } else { None }
            },
        );
        if let Some(result) = jackett {
            match result {
                Ok(trackers) => listed.extend(trackers.into_iter().map(|tracker| Tracker {
                    id: format!("jackett_{}", tracker.id).parse().expect("tracker id is valid"),
                    name: format!("{} · Jackett", tracker.name),
                })),
                Err(error) => {
                    warn!(%error, "Jackett trackers could not be listed");
                    last_error = Some(error);
                },
            }
        }
        if let Some(result) = prowlarr {
            match result {
                Ok(trackers) => listed.extend(trackers.into_iter().map(|tracker| Tracker {
                    id: format!("prowlarr_{}", tracker.id).parse().expect("tracker id is valid"),
                    name: format!("{} · Prowlarr", tracker.name),
                })),
                Err(error) => {
                    if !listed.is_empty() {
                        warn!(%error, "Prowlarr trackers could not be listed");
                    }
                    last_error = Some(error);
                },
            }
        }
        if listed.is_empty() {
            if let Some(error) = last_error {
                return Err(error);
            }
            if self.jackett_settings.current().url.is_none() && self.prowlarr_settings.current().url.is_none() {
                return Err(IndexerError::NotConfigured);
            }
        }
        Ok(listed)
    }

    async fn search(&self, query: &ReleaseQuery) -> Result<SearchResult, IndexerError> {
        let selected = match &query.trackers {
            Trackers::All => None,
            Trackers::Only(set) => Some(set.ids()),
        };
        let source_query = |prefix: &str| -> Result<Option<ReleaseQuery>, IndexerError> {
            let Some(selected) = selected else { return Ok(Some(query.clone())) };
            let ids: Vec<TrackerId> = selected
                .iter()
                .filter_map(|id| id.as_str().strip_prefix(prefix))
                .map(str::parse)
                .collect::<Result<_, _>>()
                .map_err(|_| IndexerError::Refused("Choose a valid tracker".into()))?;
            if ids.is_empty() {
                return Ok(None);
            }
            let mut copy = query.clone();
            copy.trackers = Trackers::Only(TrackerSet::new(ids).expect("selected trackers are nonempty"));
            Ok(Some(copy))
        };
        let jackett_query = source_query("jackett_")?.filter(|_| self.jackett_settings.current().url.is_some());
        let prowlarr_query = source_query("prowlarr_")?.filter(|_| self.prowlarr_settings.current().url.is_some());
        let (jackett, prowlarr) = tokio::join!(
            async { if let Some(query) = jackett_query { Some(self.jackett.search(&query).await) } else { None } },
            async { if let Some(query) = prowlarr_query { Some(self.prowlarr.search(&query).await) } else { None } },
        );
        let mut releases = Vec::new();
        let mut warnings = Vec::new();
        let mut success = false;
        let mut last_error = None;
        for (name, prefix, result) in [("Jackett", "j:", jackett), ("Prowlarr", "p:", prowlarr)] {
            if let Some(result) = result {
                match result {
                    Ok(found) => {
                        success = true;
                        warnings.extend(found.warnings);
                        releases.extend(found.releases.into_iter().map(|mut release| {
                            release.tracker = if release.tracker.is_empty() {
                                name.to_owned()
                            } else {
                                format!("{} · {name}", release.tracker)
                            };
                            release.link = format!("{prefix}{}", release.link);
                            release
                        }));
                    },
                    Err(error) => {
                        warn!(%error, "{name} search failed");
                        warnings.push(format!("{name} could not be searched"));
                        last_error = Some(error);
                    },
                }
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
            self.jackett.fetch(link).await
        } else if let Some(link) = link.strip_prefix("p:") {
            self.prowlarr.fetch(link).await
        } else {
            Err(IndexerError::Refused("Search expired; search again".into()))
        }
    }
}
