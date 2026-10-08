//! Releases found on the indexer, and adding one as a download.

use dioxus::prelude::*;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use yokoku_domain::Clock;
use yokoku_domain::{ItemId, TrackerId, Trackers};

#[cfg(feature = "server")]
use crate::api::{Dep, ReleaseSearch};

/// Releases most seeded first, and the day they are dated from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Found {
    pub releases: Vec<ReleaseEntry>,
    pub warnings: Vec<String>,
    pub today: Date,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReleaseEntry {
    pub title: String,
    pub tracker: String,
    /// Bytes.
    pub size: u64,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    /// Times it was downloaded, when the tracker counts them.
    pub grabs: Option<u32>,
    pub published: Option<Date>,
    /// Short-lived result ID that `grab_release` takes; torrent URLs stay on the server.
    pub link: String,
    /// The release's page on the tracker.
    pub details: Option<String>,
}

/// A tracker a search can be narrowed to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackerEntry {
    pub id: TrackerId,
    pub name: String,
}

/// The trackers the indexer searches, by name.
#[get("/api/releases/trackers", search: Dep<ReleaseSearch>)]
pub async fn release_trackers() -> Result<Vec<TrackerEntry>, ServerFnError> {
    server::trackers(&search).await
}

/// Searches `trackers` for `text` and `item`; for a series, `season` narrows it.
#[post("/api/releases/search", search: Dep<ReleaseSearch>, clock: Dep<dyn Clock>)]
pub async fn search_releases(
    text: String,
    item: ItemId,
    season: Option<u16>,
    trackers: Trackers,
) -> Result<Found, ServerFnError> {
    server::search(&search, &*clock, text, item, season, trackers).await
}

/// Adds the release at `link` as `add_torrent` does.
#[post("/api/releases/grab", search: Dep<ReleaseSearch>)]
pub async fn grab_release(link: String, item: ItemId, season: Option<u16>) -> Result<(), ServerFnError> {
    server::grab(&search, &link, item, season).await
}

#[cfg(feature = "server")]
mod server {
    use dioxus::prelude::*;
    use yokoku_core::downloads::{
        DownloadError, ReleaseSearch,
        ports::{IndexerError, Release, ReleaseQuery},
    };
    use yokoku_domain::{Clock, ItemId, MediaKind, Trackers};

    use super::{Found, ReleaseEntry, TrackerEntry};
    use crate::api::{downloads::add_failure, unexpected};

    pub(super) async fn trackers(search: &ReleaseSearch) -> Result<Vec<TrackerEntry>, ServerFnError> {
        let trackers = search.trackers().await.map_err(|error| failure(error, "listing trackers"))?;
        let mut entries: Vec<TrackerEntry> =
            trackers.into_iter().map(|tracker| TrackerEntry { id: tracker.id, name: tracker.name }).collect();
        entries.sort_by_key(|entry| entry.name.to_lowercase());
        Ok(entries)
    }

    pub(super) async fn search(
        search: &ReleaseSearch,
        clock: &dyn Clock,
        text: String,
        item: ItemId,
        season: Option<u16>,
        trackers: Trackers,
    ) -> Result<Found, ServerFnError> {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return Err(ServerFnError::new("Type what to search for"));
        }
        let kind = item.kind();
        let season = season.filter(|_| kind == MediaKind::Series);
        let query = ReleaseQuery { text, kind: Some(kind), season, episode: None, trackers };
        let releases = search.search(&query).await.map_err(|error| failure(error, "searching releases"))?;
        let now = clock.now();
        let zone = now.time_zone().clone();
        let entry = |release: Release| ReleaseEntry {
            published: release.published.map(|at| at.to_zoned(zone.clone()).date()),
            title: release.title,
            tracker: release.tracker,
            size: release.size,
            seeders: release.seeders,
            leechers: release.leechers,
            grabs: release.grabs,
            link: release.link,
            details: release.details,
        };
        Ok(Found {
            releases: releases.releases.into_iter().map(entry).collect(),
            warnings: releases.warnings,
            today: now.date(),
        })
    }

    pub(super) async fn grab(
        search: &ReleaseSearch,
        link: &str,
        item: ItemId,
        season: Option<u16>,
    ) -> Result<(), ServerFnError> {
        search.grab(link, Some(item), season).await.map(drop).map_err(|error| match error {
            DownloadError::Indexer(_) => failure(error, "fetching a release"),
            error => add_failure(error),
        })
    }

    fn failure(error: DownloadError, doing: &str) -> ServerFnError {
        match error {
            DownloadError::Indexer(IndexerError::NotConfigured) => ServerFnError::new(
                "Connect Jackett, add a Torznab feed, or enable an existing feed in Settings to search",
            ),
            DownloadError::Indexer(IndexerError::Unavailable(_)) => {
                ServerFnError::new("The indexers could not be reached; check their addresses")
            },
            DownloadError::Indexer(IndexerError::Refused(reason)) => ServerFnError::new(reason),
            error => unexpected(&error, doing),
        }
    }
}
