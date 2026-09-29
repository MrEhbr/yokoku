//! Changes to a library item from its page: monitoring (FR-2.1).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{MovieId, SeriesId};

#[cfg(feature = "server")]
use crate::api::{Dep, Library};

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

#[cfg(feature = "server")]
mod server {
    use dioxus::prelude::*;
    use yokoku_domain::EpisodeRef;

    use super::{Library, MonitorTarget};
    use crate::api::library_failure;

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
