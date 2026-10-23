use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use tracing::{info, instrument};
use yokoku_domain::{ItemId, Rating, StorageError, events::Event};

use crate::{
    events::{HandlerError, Subscriber},
    integrations::ports::{RatedItem, RatingsError, RatingsProvider, RatingsStore},
    media::ports::Catalog,
};

/// Keeps the library's ratings as the ratings provider gives them.
pub struct Ratings {
    provider: Arc<dyn RatingsProvider>,
    catalog: Arc<dyn Catalog>,
    store: Arc<dyn RatingsStore>,
}

#[derive(Debug, thiserror::Error)]
pub enum RatingsRefreshError {
    #[error(transparent)]
    Provider(#[from] RatingsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl Ratings {
    pub fn new(provider: Arc<dyn RatingsProvider>, catalog: Arc<dyn Catalog>, store: Arc<dyn RatingsStore>) -> Self {
        Self { provider, catalog, store }
    }

    /// The item's stored ratings.
    pub async fn of(&self, item: ItemId) -> Result<Vec<Rating>, StorageError> {
        self.store.ratings(item).await
    }

    /// Saves the provider's ratings of every item; returns how many items have one.
    #[instrument(skip_all)]
    pub async fn refresh(&self) -> Result<usize, RatingsRefreshError> {
        let series = self.catalog.all_series().await?.iter().map(RatedItem::from).collect::<Vec<_>>();
        let movies = self.catalog.all_movies().await?.iter().map(RatedItem::from).collect::<Vec<_>>();
        let items = [series, movies].concat();
        let ratings = self.provider.ratings(&items).await?;
        self.store.save_ratings(&ratings).await?;
        let rated: HashSet<ItemId> = ratings.iter().map(|&(item, _)| item).collect();
        info!(items = items.len(), rated = rated.len(), "ratings refreshed");
        Ok(rated.len())
    }
}

/// Saves the provider's ratings of each added item; does nothing when the library no longer holds it.
#[async_trait]
impl Subscriber for Ratings {
    fn name(&self) -> &'static str {
        "integrations.ratings"
    }

    async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        let rated = match event {
            Event::SeriesAdded(added) => self.catalog.series(added.series).await?.as_ref().map(RatedItem::from),
            Event::MovieAdded(added) => self.catalog.movie(added.movie).await?.as_ref().map(RatedItem::from),
            _ => return Ok(()),
        };
        let Some(rated) = rated else { return Ok(()) };
        let ratings = self.provider.ratings(&[rated]).await?;
        self.store.save_ratings(&ratings).await?;
        Ok(())
    }
}
