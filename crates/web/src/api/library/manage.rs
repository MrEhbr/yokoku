//! Changes to a library item from its page: monitoring, refresh, numbering, removal and deleting
//! files.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ItemId, MediaFileId, MovieId, SeriesId};

use super::detail::{Numbering, Watched};
#[cfg(feature = "server")]
use crate::api::{Deleter, Dep, Library, MetadataService};

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

/// Reads the item's metadata from its source again.
#[post("/api/items/refresh", metadata: Dep<MetadataService>)]
pub async fn refresh(item: ItemId) -> Result<(), ServerFnError> {
    server::refresh(&metadata, item).await
}

/// Sets how the series' episode numbers are read from file names.
#[post("/api/series/{id}/numbering", library: Dep<Library>)]
pub async fn set_numbering(id: SeriesId, numbering: Numbering) -> Result<(), ServerFnError> {
    server::set_numbering(&library, id, numbering).await
}

/// Removes the item from the library after deleting `delete`, files of it; its other files stay
/// on disk.
#[post("/api/items/remove", library: Dep<Library>, deleter: Dep<Deleter>)]
pub async fn remove(item: ItemId, delete: Vec<MediaFileId>) -> Result<(), ServerFnError> {
    server::remove(&library, &deleter, item, delete).await
}

/// One library file of an item, to choose what to delete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemFile {
    pub id: MediaFileId,
    /// `None` for a movie.
    pub season: Option<u16>,
    /// `S01E01`, or `S01E01–E02` for a file of several; empty for a movie.
    pub episodes: String,
    /// The episodes' titles, or the movie's.
    pub title: String,
    /// Bytes.
    pub size: u64,
    pub watched: Option<Watched>,
}

/// The item's library files, by season and episode.
#[get("/api/items/files?item", library: Dep<Library>, deleter: Dep<Deleter>)]
pub async fn item_files(item: ItemId) -> Result<Vec<ItemFile>, ServerFnError> {
    server::item_files(&library, &deleter, item).await
}

/// Deletes `files` of `item` from disk with their subtitles, and stops monitoring the
/// episodes or movie they held when `unmonitor`.
#[post("/api/items/delete-files", library: Dep<Library>, deleter: Dep<Deleter>)]
pub async fn delete_files(item: ItemId, files: Vec<MediaFileId>, unmonitor: bool) -> Result<(), ServerFnError> {
    server::delete_files(&library, &deleter, item, files, unmonitor).await
}

#[cfg(feature = "server")]
mod server {
    use std::collections::HashMap;

    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_core::media::MediaError;
    use yokoku_domain::{EpisodeRef, FileTarget, ItemId, MediaFileId, SeriesId};

    use super::{Deleter, ItemFile, Library, MetadataService, MonitorTarget, Numbering, Watched};
    use crate::{api::library_failure, format::episode};

    /// Deleting every file goes with the removal, so history names it.
    pub(super) async fn remove(
        library: &Library,
        deleter: &Deleter,
        item: ItemId,
        delete: Vec<MediaFileId>,
    ) -> Result<(), ServerFnError> {
        let all = deleter.files_of_item(item).await.map_err(|error| media_failure(error, item))?;
        let delete_all = !all.is_empty() && all.iter().all(|file| delete.contains(&file.id));
        if !delete_all && !delete.is_empty() {
            match deleter.delete_files(item, &delete).await {
                Ok(_) | Err(MediaError::NoFile) => {},
                Err(error) => return Err(media_failure(error, item)),
            }
        }
        let removed = match item {
            ItemId::Series(id) => library.remove_series(id, delete_all).await,
            ItemId::Movie(id) => library.remove_movie(id, delete_all).await,
        };
        removed.map_err(|error| library_failure(error, "removing the item"))
    }

    pub(super) async fn item_files(
        library: &Library,
        deleter: &Deleter,
        item: ItemId,
    ) -> Result<Vec<ItemFile>, ServerFnError> {
        let mut files = deleter.files_of_item(item).await.map_err(|error| media_failure(error, item))?;
        files.sort_by_key(|file| match file.target {
            FileTarget::Episodes { span, .. } => (span.season(), span.first(), span.last()),
            FileTarget::Movie(_) => (0, 0, 0),
        });
        let watched = library.watched_files().await.map_err(|error| library_failure(error, "reading watched files"))?;
        let titles: HashMap<EpisodeRef, String> = match item {
            ItemId::Series(id) => {
                let series = library.series(id).await.map_err(|error| library_failure(error, "reading the series"))?;
                series
                    .seasons
                    .iter()
                    .flat_map(|season| {
                        season.episodes.iter().map(|episode| {
                            (EpisodeRef { season: season.number, episode: episode.number }, episode.title.clone())
                        })
                    })
                    .collect()
            },
            ItemId::Movie(_) => HashMap::new(),
        };
        let movie_title = match item {
            ItemId::Movie(id) => {
                library.movie(id).await.map_err(|error| library_failure(error, "reading the movie"))?.title
            },
            ItemId::Series(_) => String::new(),
        };
        let files = files
            .into_iter()
            .map(|file| {
                let (season, episodes, title) = match file.target {
                    FileTarget::Episodes { span, .. } => {
                        let first = episode(span.season(), span.first());
                        let episodes =
                            if span.first() == span.last() { first } else { format!("{first}–E{:02}", span.last()) };
                        let title = (span.first()..=span.last())
                            .filter_map(|number| titles.get(&EpisodeRef { season: span.season(), episode: number }))
                            .filter(|title| !title.is_empty())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(" / ");
                        (Some(span.season()), episodes, title)
                    },
                    FileTarget::Movie(_) => (None, String::new(), movie_title.clone()),
                };
                let watched = watched.get(&file.id).map(|at| Watched { on: at.map(|at| library.date_of(at)) });
                ItemFile { id: file.id, season, episodes, title, size: file.size, watched }
            })
            .collect();
        Ok(files)
    }

    pub(super) async fn delete_files(
        library: &Library,
        deleter: &Deleter,
        item: ItemId,
        files: Vec<MediaFileId>,
        unmonitor: bool,
    ) -> Result<(), ServerFnError> {
        let deleted = deleter.delete_files(item, &files).await.map_err(|error| media_failure(error, item))?;
        if unmonitor {
            let targets: Vec<_> = deleted.iter().map(|file| file.target).collect();
            library.stop_monitoring(&targets).await.map_err(|error| library_failure(error, "changing monitoring"))?;
        }
        Ok(())
    }

    fn media_failure(error: MediaError, item: ItemId) -> ServerFnError {
        match error {
            MediaError::NoFile => ServerFnError::new("Those files are gone already; reload the page"),
            error => {
                error!(%error, ?item, "deleting files failed");
                ServerFnError::new("The files could not be deleted; the server log has the cause")
            },
        }
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
