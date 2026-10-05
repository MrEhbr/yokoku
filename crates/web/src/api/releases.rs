//! Releases found on the indexer, and adding one as a download.

use dioxus::prelude::*;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use yokoku_domain::Clock;
use yokoku_domain::ItemId;

#[cfg(feature = "server")]
use crate::api::{Dep, ReleaseSearch};

/// Releases most seeded first, and the day they are dated from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Found {
    pub releases: Vec<ReleaseEntry>,
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
    /// What `grab_release` takes.
    pub link: String,
    /// The release's page on the tracker.
    pub details: Option<String>,
}

/// Searches the indexer for `text`, in the categories of `item` when given; for a series,
/// `season` narrows it.
#[post("/api/releases/search", search: Dep<ReleaseSearch>, clock: Dep<dyn Clock>)]
pub async fn search_releases(text: String, item: Option<ItemId>, season: Option<u16>) -> Result<Found, ServerFnError> {
    server::search(&search, &*clock, text, item, season).await
}

/// Adds the release at `link` as `add_torrent` does.
#[post("/api/releases/grab", search: Dep<ReleaseSearch>)]
pub async fn grab_release(link: String, item: Option<ItemId>, season: Option<u16>) -> Result<(), ServerFnError> {
    server::grab(&search, &link, item, season).await
}

#[cfg(feature = "server")]
mod server {
    use dioxus::prelude::*;
    use yokoku_core::downloads::{
        DownloadError, ReleaseSearch,
        ports::{IndexerError, Release, ReleaseQuery},
    };
    use yokoku_domain::{Clock, ItemId, MediaKind};

    use super::{Found, ReleaseEntry};
    use crate::api::{downloads::add_failure, unexpected};

    pub(super) async fn search(
        search: &ReleaseSearch,
        clock: &dyn Clock,
        text: String,
        item: Option<ItemId>,
        season: Option<u16>,
    ) -> Result<Found, ServerFnError> {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return Err(ServerFnError::new("Type what to search for"));
        }
        let kind = item.map(ItemId::kind);
        let season = season.filter(|_| kind == Some(MediaKind::Series));
        let query = ReleaseQuery { text, kind, season, episode: None };
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
        Ok(Found { releases: releases.into_iter().map(entry).collect(), today: now.date() })
    }

    pub(super) async fn grab(
        search: &ReleaseSearch,
        link: &str,
        item: Option<ItemId>,
        season: Option<u16>,
    ) -> Result<(), ServerFnError> {
        search.grab(link, item, season).await.map(drop).map_err(|error| match error {
            DownloadError::Indexer(_) => failure(error, "fetching a release"),
            error => add_failure(error),
        })
    }

    fn failure(error: DownloadError, doing: &str) -> ServerFnError {
        match error {
            DownloadError::Indexer(IndexerError::NotConfigured) => {
                ServerFnError::new("Set the Jackett address in Settings to search")
            },
            DownloadError::Indexer(IndexerError::Unavailable(_)) => {
                ServerFnError::new("Jackett could not be reached; check that it runs and its address")
            },
            DownloadError::Indexer(IndexerError::Refused(reason)) => {
                ServerFnError::new(format!("Jackett refused: {reason}"))
            },
            error => unexpected(&error, doing),
        }
    }
}
