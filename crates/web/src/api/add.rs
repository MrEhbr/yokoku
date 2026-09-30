//! Searching the metadata source and adding a result to the library (FR-1.1, 2.2, 8.1).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ItemId, MonitorPreset};

use super::library::Kind;
#[cfg(feature = "server")]
use super::{AddSettings, Dep, MetadataService, RootFolders};

/// A search result; `source` is its id at the metadata source, like `tmdb:438631`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: Kind,
    pub source: String,
    pub title: String,
    /// Left out when it is the title.
    pub original_title: Option<String>,
    pub year: Option<i16>,
    /// The poster's URL, through this server.
    pub poster: Option<String>,
    /// Empty when the source has none.
    pub overview: String,
    /// The folder name the naming pattern gives it.
    pub folder: String,
    /// The item holding this result when it is already in the library.
    pub in_library: Option<ItemId>,
}

/// What the add form offers, whichever result is picked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddOptions {
    pub series_roots: Vec<RootChoice>,
    pub movie_roots: Vec<RootChoice>,
    pub monitor: MonitorPreset,
}

impl AddOptions {
    pub fn roots(&self, kind: Kind) -> &[RootChoice] {
        match kind {
            Kind::Series => &self.series_roots,
            Kind::Movie => &self.movie_roots,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RootChoice {
    pub path: String,
    /// Names of the folders already in it.
    pub folders: Vec<String>,
    /// Names of its items' folders, which a new item cannot take.
    pub taken: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewItem {
    pub kind: Kind,
    pub source: String,
    pub root: String,
    /// Movies take `All` or `None`.
    pub monitor: MonitorPreset,
    pub folder: String,
}

/// Series or movies matching `query`.
#[get("/api/search?query&kind", metadata: Dep<MetadataService>, settings: Dep<AddSettings>)]
pub async fn search(query: String, kind: Kind) -> Result<Vec<SearchHit>, ServerFnError> {
    server::search(&metadata, &settings, &query, kind).await
}

#[get("/api/add-options", roots: Dep<RootFolders>, settings: Dep<AddSettings>)]
pub async fn add_options() -> Result<AddOptions, ServerFnError> {
    server::add_options(&roots, &settings).await
}

/// Adds the item and answers with its id.
#[post("/api/items", metadata: Dep<MetadataService>, roots: Dep<RootFolders>, settings: Dep<AddSettings>)]
pub async fn add_item(item: NewItem) -> Result<ItemId, ServerFnError> {
    server::add_item(&metadata, &roots, &settings, item).await
}

#[cfg(feature = "server")]
mod server {
    use std::path::Path;

    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_domain::{ArtworkKind, ExternalId, ItemId, MonitorPreset};
    use yokoku_media::{MediaError, RootKind};

    use super::{AddOptions, AddSettings, Kind, MetadataService, NewItem, RootChoice, RootFolders, SearchHit};
    use crate::api::{artwork, library_failure, root_listing_failed};

    pub(super) async fn search(
        metadata: &MetadataService,
        settings: &AddSettings,
        query: &str,
        kind: Kind,
    ) -> Result<Vec<SearchHit>, ServerFnError> {
        ready(settings)?;
        let hits = metadata
            .search(query.trim(), Some(kind.into()))
            .await
            .map_err(|error| library_failure(error, "searching"))?;
        Ok(hits
            .into_iter()
            .map(|hit| {
                let result = hit.result;
                SearchHit {
                    kind: result.kind.into(),
                    source: result.source.to_string(),
                    original_title: (result.original_title != result.title).then_some(result.original_title),
                    title: result.title,
                    year: result.year,
                    poster: result
                        .poster_path
                        .map(|path| artwork::preview_url(result.source, ArtworkKind::Poster, &path)),
                    overview: result.overview,
                    folder: hit.folder,
                    in_library: hit.in_library,
                }
            })
            .collect())
    }

    pub(super) async fn add_options(roots: &RootFolders, settings: &AddSettings) -> Result<AddOptions, ServerFnError> {
        let mut options =
            AddOptions { series_roots: Vec::new(), movie_roots: Vec::new(), monitor: settings.monitor.current() };
        for root in roots.list().await.map_err(root_listing_failed)? {
            let folders = roots.folders(&root).await.map_err(root_listing_failed)?;
            let taken = roots.item_folders(&root).await.map_err(root_listing_failed)?;
            let choice = RootChoice { path: root.path.display().to_string(), folders, taken };
            match root.kind {
                RootKind::Series => options.series_roots.push(choice),
                RootKind::Movies => options.movie_roots.push(choice),
            }
        }
        Ok(options)
    }

    pub(super) async fn add_item(
        metadata: &MetadataService,
        roots: &RootFolders,
        settings: &AddSettings,
        item: NewItem,
    ) -> Result<ItemId, ServerFnError> {
        ready(settings)?;
        let source = parse(&item.source)?;
        let root = roots.get(item.kind.into(), Path::new(&item.root)).await.map_err(|error| match error {
            MediaError::RootNotFound(_) | MediaError::WrongRootKind { .. } => {
                ServerFnError::new(format!("Pick a root folder for {}", RootKind::from(item.kind).as_str()))
            },
            error => {
                error!(%error, "reading root folders failed");
                ServerFnError::new("The root folders could not be loaded")
            },
        })?;
        let folder = Some(item.folder.trim().to_owned());
        let added = match item.kind {
            Kind::Series => metadata
                .add_series(source, item.monitor, root.path, folder)
                .await
                .map(|series| ItemId::Series(series.id)),
            Kind::Movie => {
                let monitored = match item.monitor {
                    MonitorPreset::All => true,
                    MonitorPreset::None => false,
                    MonitorPreset::Future | MonitorPreset::LatestSeason => {
                        return Err(ServerFnError::new("Movies are either monitored or not"));
                    },
                };
                metadata.add_movie(source, monitored, root.path, folder).await.map(|movie| ItemId::Movie(movie.id))
            },
        };
        added.map_err(|error| library_failure(error, "adding the item"))
    }

    fn ready(settings: &AddSettings) -> Result<(), ServerFnError> {
        if settings.tmdb_token_set.current() {
            Ok(())
        } else {
            Err(ServerFnError::new("Set a TMDB token in APP__METADATA__TMDB__TOKEN to search and add items"))
        }
    }

    fn parse(source: &str) -> Result<ExternalId, ServerFnError> {
        source.parse().map_err(|_| ServerFnError::new(format!("{source:?} is not a TMDB or TVDB id")))
    }
}
