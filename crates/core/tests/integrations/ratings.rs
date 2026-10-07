use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use jiff::{Timestamp, civil::date};
use yokoku_core::{
    events::Handler,
    integrations::{
        Ratings,
        ports::{RatedItem, RatingsError, RatingsProvider, RatingsStore},
    },
    library::ports::{MovieRepo, SeriesRepo},
};
use yokoku_domain::{
    ExternalId, ExternalIds, ItemFolder, ItemId, MonitorPreset, Movie, MovieId, Rating, RatingSource, Releases, Series,
    SourceStatus, events::MovieAdded,
};
use yokoku_infra::db::Database;
use yokoku_test_support::metadata::{movie_metadata, series_metadata};

/// Rates every item it is asked about at `value`, or fails; records what it was asked.
struct FakeProvider {
    value: Mutex<Option<f32>>,
    asked: Mutex<Vec<Vec<RatedItem>>>,
}

#[async_trait]
impl RatingsProvider for FakeProvider {
    async fn ratings(&self, items: &[RatedItem]) -> Result<Vec<(ItemId, Rating)>, RatingsError> {
        self.asked.lock().unwrap().push(items.to_vec());
        let value = self.value.lock().unwrap().ok_or_else(|| RatingsError::Unavailable("offline".into()))?;
        Ok(items.iter().map(|rated| (rated.item, imdb(value))).collect())
    }
}

fn imdb(value: f32) -> Rating {
    Rating { source: RatingSource::Imdb, value, votes: Some(1000) }
}

/// "Frieren" with an IMDb id and "Dune" without one.
struct Setup {
    db: Database,
    provider: Arc<FakeProvider>,
    ratings: Ratings,
    frieren: Series,
    dune: Movie,
}

async fn setup() -> Setup {
    let db = Database::open_in_memory().await.unwrap();
    let mut metadata = series_metadata(209867, "Frieren", SourceStatus::Returning, &[]);
    metadata.external_ids.imdb = Some("tt22248376".parse().unwrap());
    let mut frieren = Series::new(
        metadata,
        ItemFolder::new("/tv".into(), "Frieren (2023)".into()).unwrap(),
        MonitorPreset::All,
        date(2023, 9, 29),
        Timestamp::UNIX_EPOCH,
    );
    let mut dune = Movie::new(
        movie_metadata(438631, "Dune", Releases::default()),
        ItemFolder::new("/movies".into(), "Dune (2021)".into()).unwrap(),
        true,
        Timestamp::UNIX_EPOCH,
    );
    SeriesRepo::save(&db, &mut frieren).await.unwrap();
    MovieRepo::save(&db, &mut dune).await.unwrap();

    let provider = Arc::new(FakeProvider { value: Mutex::new(Some(8.0)), asked: Mutex::new(Vec::new()) });
    let store = Arc::new(db.clone());
    let ratings = Ratings::new(provider.clone(), store.clone(), store);
    Setup { db, provider, ratings, frieren, dune }
}

impl Setup {
    fn rate(&self, value: Option<f32>) {
        *self.provider.value.lock().unwrap() = value;
    }

    fn asked(&self) -> Vec<Vec<RatedItem>> {
        self.provider.asked.lock().unwrap().clone()
    }

    async fn stored(&self, item: ItemId) -> Vec<Rating> {
        self.db.ratings(item).await.unwrap()
    }
}

#[tokio::test]
async fn a_refresh_asks_about_every_item_by_its_ids_and_stores_the_answers() {
    let setup = setup().await;
    let (frieren, dune) = (ItemId::Series(setup.frieren.id), ItemId::Movie(setup.dune.id));

    let rated = setup.ratings.refresh().await.unwrap();

    assert_eq!(rated, 2);
    assert_eq!(
        setup.asked(),
        [vec![
            RatedItem {
                item: frieren,
                source: ExternalId::Tmdb(209867),
                external_ids: ExternalIds { imdb: Some("tt22248376".parse().unwrap()) }
            },
            RatedItem { item: dune, source: ExternalId::Tmdb(438631), external_ids: ExternalIds::default() },
        ]]
    );
    assert_eq!(setup.stored(frieren).await, [imdb(8.0)]);
    assert_eq!(setup.ratings.of(dune).await.unwrap(), [imdb(8.0)]);
}

#[tokio::test]
async fn a_failed_refresh_keeps_the_stored_ratings() {
    let setup = setup().await;
    let frieren = ItemId::Series(setup.frieren.id);
    setup.ratings.refresh().await.unwrap();
    setup.rate(None);

    assert!(setup.ratings.refresh().await.is_err());
    assert!(setup.ratings.refresh_item(frieren).await.is_err());

    assert_eq!(setup.stored(frieren).await, [imdb(8.0)]);
}

#[tokio::test]
async fn refreshing_one_item_leaves_the_others() {
    let setup = setup().await;
    let (frieren, dune) = (ItemId::Series(setup.frieren.id), ItemId::Movie(setup.dune.id));
    setup.ratings.refresh().await.unwrap();
    setup.rate(Some(9.0));

    setup.ratings.refresh_item(dune).await.unwrap();

    assert_eq!(setup.stored(frieren).await, [imdb(8.0)]);
    assert_eq!(setup.stored(dune).await, [imdb(9.0)]);
}

#[tokio::test]
async fn an_added_item_is_rated() {
    let setup = setup().await;
    let dune = ItemId::Movie(setup.dune.id);

    setup.ratings.handle(&MovieAdded { movie: setup.dune.id, title: "Dune".into() }).await.unwrap();

    assert_eq!(setup.stored(dune).await, [imdb(8.0)]);
    assert_eq!(setup.asked().concat().iter().map(|rated| rated.item).collect::<Vec<_>>(), [dune]);
}

#[tokio::test]
async fn an_item_no_longer_in_the_library_is_not_asked_about() {
    let setup = setup().await;

    setup.ratings.handle(&MovieAdded { movie: MovieId::generate(), title: "Gone".into() }).await.unwrap();

    assert_eq!(setup.asked(), Vec::<Vec<RatedItem>>::new());
}
