//! Changes to a library item from its page: monitoring (FR-2.1), refresh (FR-1.6), numbering
//! (FR-1.8) and removal (FR-1.7).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ItemId, MovieId, SeriesId};

use super::detail::Numbering;
#[cfg(feature = "server")]
use crate::api::{Dep, Library, MetadataService};

/// What a monitoring change applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum MonitorTarget {
    Series { id: SeriesId },
    Season { id: SeriesId, season: u16 },
    Episode { id: SeriesId, season: u16, episode: u16 },
    Movie { id: MovieId },
}

#[post("/api/monitoring", library: Dep<Library>)]
pub async fn set_monitored(target: MonitorTarget, monitored: bool) -> Result<(), ServerFnError> {
    server::set_monitored(&library, target, monitored).await
}

/// Reads the item's metadata from its source again (FR-1.6).
#[post("/api/items/refresh", metadata: Dep<MetadataService>)]
pub async fn refresh(item: ItemId) -> Result<(), ServerFnError> {
    server::refresh(&metadata, item).await
}

/// Sets how the series' episode numbers are read from file names (FR-1.8).
#[post("/api/series/{id}/numbering", library: Dep<Library>)]
pub async fn set_numbering(id: SeriesId, numbering: Numbering) -> Result<(), ServerFnError> {
    server::set_numbering(&library, id, numbering).await
}

/// Removes the item from the library, and its files from disk when `delete_files` (FR-1.7).
#[post("/api/items/remove", library: Dep<Library>)]
pub async fn remove(item: ItemId, delete_files: bool) -> Result<(), ServerFnError> {
    server::remove(&library, item, delete_files).await
}

#[cfg(feature = "server")]
mod server {
    use dioxus::prelude::*;
    use yokoku_domain::{EpisodeRef, ItemId, SeriesId};

    use super::{Library, MetadataService, MonitorTarget, Numbering};
    use crate::api::library_failure;

    pub(super) async fn remove(library: &Library, item: ItemId, delete_files: bool) -> Result<(), ServerFnError> {
        let removed = match item {
            ItemId::Series(id) => library.remove_series(id, delete_files).await,
            ItemId::Movie(id) => library.remove_movie(id, delete_files).await,
        };
        removed.map_err(|error| library_failure(error, "removing the item"))
    }

    impl From<Numbering> for yokoku_domain::Numbering {
        fn from(numbering: Numbering) -> Self {
            match numbering {
                Numbering::Standard => Self::Standard,
                Numbering::Absolute => Self::Absolute,
            }
        }
    }

    pub(super) async fn set_numbering(
        library: &Library,
        id: SeriesId,
        numbering: Numbering,
    ) -> Result<(), ServerFnError> {
        library.set_numbering(id, numbering.into()).await.map_err(|error| library_failure(error, "changing numbering"))
    }

    pub(super) async fn refresh(metadata: &MetadataService, item: ItemId) -> Result<(), ServerFnError> {
        let refreshed = match item {
            ItemId::Series(id) => metadata.refresh_series(id).await.map(drop),
            ItemId::Movie(id) => metadata.refresh_movie(id).await.map(drop),
        };
        refreshed.map_err(|error| library_failure(error, "refreshing the item"))
    }

    pub(super) async fn set_monitored(
        library: &Library,
        target: MonitorTarget,
        monitored: bool,
    ) -> Result<(), ServerFnError> {
        let changed = match target {
            MonitorTarget::Series { id } => library.set_series_monitored(id, monitored).await,
            MonitorTarget::Season { id, season } => library.set_season_monitored(id, season, monitored).await,
            MonitorTarget::Episode { id, season, episode } => {
                library.set_episode_monitored(id, EpisodeRef { season, episode }, monitored).await
            },
            MonitorTarget::Movie { id } => library.set_movie_monitored(id, monitored).await,
        };
        changed.map_err(|error| library_failure(error, "changing monitoring"))
    }
}
