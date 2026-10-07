use jiff::civil::date;
use rstest::rstest;
use support::{db, now};
use yokoku_core::{
    integrations::ports::RatingsStore,
    library::ports::{MovieRepo, SeriesRepo},
};
use yokoku_domain::{
    ItemFolder, ItemId, MonitorPreset, Movie, MovieId, Rating, RatingSource, Releases, Series, SeriesId, SourceStatus,
};
use yokoku_infra::db::Database;
use yokoku_test_support::metadata::{movie_metadata, series_metadata};

use crate::support;

fn imdb(value: f32, votes: u32) -> Rating {
    Rating { source: RatingSource::Imdb, value, votes: Some(votes) }
}

async fn stored_series(db: &Database, source: u64) -> ItemId {
    let metadata = series_metadata(source, "Frieren", SourceStatus::Returning, &[]);
    let folder = ItemFolder::new("/tv".into(), format!("Series {source}")).unwrap();
    let mut series = Series::new(metadata, folder, MonitorPreset::All, date(2026, 9, 26), now());
    SeriesRepo::save(db, &mut series).await.unwrap();
    ItemId::Series(series.id)
}

async fn stored_movie(db: &Database, source: u64) -> ItemId {
    let folder = ItemFolder::new("/movies".into(), format!("Movie {source}")).unwrap();
    let mut movie = Movie::new(movie_metadata(source, "Dune", Releases::default()), folder, true, now());
    MovieRepo::save(db, &mut movie).await.unwrap();
    ItemId::Movie(movie.id)
}

#[rstest]
#[tokio::test]
async fn a_saved_rating_replaces_the_items_one_from_the_same_source(#[future(awt)] db: Database) {
    let (series, movie, other) = (stored_series(&db, 1).await, stored_movie(&db, 2).await, stored_movie(&db, 3).await);
    db.save_ratings(&[(series, imdb(8.0, 10)), (other, imdb(6.0, 20))]).await.unwrap();

    db.save_ratings(&[(series, imdb(9.1, 300)), (movie, imdb(7.8, 1_200_000))]).await.unwrap();

    assert_eq!(db.ratings(series).await.unwrap(), [imdb(9.1, 300)]);
    assert_eq!(db.ratings(movie).await.unwrap(), [imdb(7.8, 1_200_000)]);
    assert_eq!(db.ratings(other).await.unwrap(), [imdb(6.0, 20)]);
}

#[rstest]
#[tokio::test]
async fn a_rating_of_an_item_no_longer_stored_is_skipped(#[future(awt)] db: Database) {
    let movie = stored_movie(&db, 1).await;
    let gone = ItemId::Movie(MovieId::generate());

    db.save_ratings(&[(gone, imdb(5.0, 1)), (movie, imdb(7.8, 1000))]).await.unwrap();

    assert_eq!(db.ratings(gone).await.unwrap(), []);
    assert_eq!(db.ratings(movie).await.unwrap(), [imdb(7.8, 1000)]);
}

#[rstest]
#[tokio::test]
async fn a_rating_without_votes_loads_back(#[future(awt)] db: Database) {
    let movie = stored_movie(&db, 1).await;
    let rating = Rating { votes: None, ..imdb(7.5, 0) };

    db.save_ratings(&[(movie, rating)]).await.unwrap();

    assert_eq!(db.ratings(movie).await.unwrap(), [rating]);
}

#[rstest]
#[tokio::test]
async fn ratings_go_with_their_item(#[future(awt)] db: Database) {
    let (series, movie) = (stored_series(&db, 1).await, stored_movie(&db, 2).await);
    db.save_ratings(&[(series, imdb(9.1, 300)), (movie, imdb(7.8, 1000))]).await.unwrap();

    SeriesRepo::remove(&db, series.series().unwrap()).await.unwrap();
    MovieRepo::remove(&db, movie.movie().unwrap()).await.unwrap();

    assert_eq!(db.ratings(series).await.unwrap(), []);
    assert_eq!(db.ratings(movie).await.unwrap(), []);
    assert_eq!(db.ratings(ItemId::Series(SeriesId::generate())).await.unwrap(), []);
}

#[rstest]
#[tokio::test]
async fn saving_an_item_keeps_its_ratings(#[future(awt)] db: Database) {
    let (series, movie) = (stored_series(&db, 1).await, stored_movie(&db, 2).await);
    db.save_ratings(&[(series, imdb(9.1, 300)), (movie, imdb(7.8, 1000))]).await.unwrap();

    let mut stored_series = SeriesRepo::get(&db, series.series().unwrap()).await.unwrap().unwrap();
    stored_series.monitored = false;
    SeriesRepo::save(&db, &mut stored_series).await.unwrap();
    let mut stored_movie = MovieRepo::get(&db, movie.movie().unwrap()).await.unwrap().unwrap();
    stored_movie.monitored = false;
    MovieRepo::save(&db, &mut stored_movie).await.unwrap();

    assert_eq!(db.ratings(series).await.unwrap(), [imdb(9.1, 300)]);
    assert_eq!(db.ratings(movie).await.unwrap(), [imdb(7.8, 1000)]);
}
