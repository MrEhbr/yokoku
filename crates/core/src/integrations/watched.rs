use std::{collections::HashMap, hash::Hash, path::Path, sync::Arc};

use jiff::Timestamp;
use tracing::{debug, info, instrument};
use yokoku_domain::{ExternalId, FileTarget, ItemId, StorageError};

use crate::{
    integrations::ports::{MediaServer, MediaServerError, Played, PlayedItem, Watched, WatchedStore},
    media::{
        MediaFile,
        ports::{Catalog, MediaRepo},
    },
};

/// Mirrors the media server user's played items onto library files.
pub struct WatchSync {
    server: Arc<dyn MediaServer>,
    catalog: Arc<dyn Catalog>,
    media: Arc<dyn MediaRepo>,
    store: Arc<dyn WatchedStore>,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchSyncError {
    #[error(transparent)]
    Server(#[from] MediaServerError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl WatchSync {
    pub fn new(
        server: Arc<dyn MediaServer>,
        catalog: Arc<dyn Catalog>,
        media: Arc<dyn MediaRepo>,
        store: Arc<dyn WatchedStore>,
    ) -> Self {
        Self { server, catalog, media, store }
    }

    /// Replaces the watched files with the library files the user has played: one at the same path,
    /// or else one whose series or movie and episodes the media server knows it by. Leaves them as
    /// they are while no media server or user is configured.
    #[instrument(skip_all)]
    pub async fn sync(&self) -> Result<(), WatchSyncError> {
        let played = match self.server.played().await {
            Err(MediaServerError::NotConfigured) => {
                debug!("watched sync skipped: no media server user");
                return Ok(());
            },
            result => result?,
        };
        let sources = self.sources().await?;
        let index = PlayedIndex::new(&played);
        let watched: Vec<Watched> = self
            .media
            .files()
            .await?
            .iter()
            .filter_map(|file| index.of(file, &sources).map(|at| Watched { file: file.id, at }))
            .collect();
        self.store.replace_watched(&watched).await?;
        info!(played = played.len(), watched = watched.len(), "watched files synced");
        Ok(())
    }

    /// Each series' and movie's id at its metadata source.
    async fn sources(&self) -> Result<HashMap<ItemId, ExternalId>, StorageError> {
        let series =
            self.catalog.all_series().await?.into_iter().map(|series| (ItemId::Series(series.id), series.source));
        let movies = self.catalog.all_movies().await?.into_iter().map(|movie| (ItemId::Movie(movie.id), movie.source));
        Ok(series.chain(movies).collect())
    }
}

/// When each played path, episode and movie was last played.
struct PlayedIndex<'a> {
    paths: HashMap<&'a Path, Option<Timestamp>>,
    /// Keyed by series id, season and episode number.
    episodes: HashMap<(ExternalId, u16, u16), Option<Timestamp>>,
    movies: HashMap<ExternalId, Option<Timestamp>>,
}

impl<'a> PlayedIndex<'a> {
    fn new(played: &'a [Played]) -> Self {
        let mut index = Self { paths: HashMap::new(), episodes: HashMap::new(), movies: HashMap::new() };
        for played in played {
            latest(&mut index.paths, played.path.as_path(), played.at);
            match &played.item {
                Some(PlayedItem::Episodes { series, span }) => {
                    for &id in series {
                        for episode in span.first()..=span.last() {
                            latest(&mut index.episodes, (id, span.season(), episode), played.at);
                        }
                    }
                },
                Some(PlayedItem::Movie(ids)) => ids.iter().for_each(|&id| latest(&mut index.movies, id, played.at)),
                None => {},
            }
        }
        index
    }

    /// `Some` with when `file` was last played, if it was: by its path, or else every episode it holds.
    fn of(&self, file: &MediaFile, sources: &HashMap<ItemId, ExternalId>) -> Option<Option<Timestamp>> {
        if let Some(&at) = self.paths.get(file.path.as_path()) {
            return Some(at);
        }
        let source = *sources.get(&file.target.item())?;
        match file.target {
            FileTarget::Episodes { span, .. } => (span.first()..=span.last())
                .map(|episode| self.episodes.get(&(source, span.season(), episode)).copied())
                .try_fold(None, |latest, at| Some(latest.max(at?))),
            FileTarget::Movie(_) => self.movies.get(&source).copied(),
        }
    }
}

/// Keeps the later of the time stored under `key` and `at`, a known time over none.
fn latest<K: Eq + Hash>(map: &mut HashMap<K, Option<Timestamp>>, key: K, at: Option<Timestamp>) {
    let entry = map.entry(key).or_insert(at);
    *entry = (*entry).max(at);
}
