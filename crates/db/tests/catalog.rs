use std::collections::BTreeMap;

use jiff::{
    Timestamp, ToSpan,
    civil::{Date, date},
};
use proptest::prelude::*;
use rstest::{fixture, rstest};
use yokoku_db::Database;
use yokoku_domain::{
    EpisodeMetadata, ExternalId, MediaFileId, MonitorPreset, Movie, MovieMetadata, Numbering, Releases, SeasonMetadata,
    Series, SeriesMetadata, SourceStatus,
};
use yokoku_events::{Event, EventLog};
use yokoku_library::ports::{MovieRepo, SeriesRepo, StorageError};

const TODAY: Date = date(2026, 9, 26);

#[fixture]
async fn db() -> Database {
    Database::open_in_memory().await.unwrap()
}

fn now() -> Timestamp {
    "2026-09-26T12:00:00.123456789Z".parse().unwrap()
}

/// Seasons as `(number, air dates)`; source ids are assigned in order, so two calls share ids.
fn series_metadata(source: u64, seasons: &[(u16, &[Option<Date>])]) -> SeriesMetadata {
    let mut next_source_id = 1000;
    SeriesMetadata {
        source: ExternalId::Tmdb(source),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        year: Some(2023),
        poster_path: Some("/frieren.jpg".into()),
        status: SourceStatus::Returning,
        seasons: seasons
            .iter()
            .map(|&(number, dates)| SeasonMetadata {
                number,
                episodes: dates
                    .iter()
                    .zip(1..)
                    .map(|(&air_date, episode)| {
                        next_source_id += 1;
                        EpisodeMetadata {
                            source_id: next_source_id,
                            number: episode,
                            title: format!("Episode {episode}"),
                            air_date,
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn movie_metadata(source: u64) -> MovieMetadata {
    MovieMetadata {
        source: ExternalId::Tmdb(source),
        title: "Dune".into(),
        original_title: "Dune".into(),
        year: None,
        poster_path: None,
        releases: Releases { cinema: Some(TODAY), digital: None, physical: Some(TODAY + 90.days()) },
    }
}

#[rstest]
#[tokio::test]
async fn saved_series_loads_back_equal(#[future(awt)] db: Database) {
    let mut series = Series::add(
        series_metadata(1, &[(0, &[None]), (1, &[Some(TODAY), None])]),
        MonitorPreset::Future,
        TODAY,
        now(),
    );
    series.numbering = Numbering::Absolute;
    series.seasons[1].episodes[0].file = Some(MediaFileId::generate());

    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), Some(series.clone()));
    assert_eq!(SeriesRepo::find_by_source(&db, ExternalId::Tmdb(1)).await.unwrap(), Some(series.clone()));
    assert_eq!(SeriesRepo::find_by_source(&db, ExternalId::Tvdb(1)).await.unwrap(), None);
    assert_eq!(SeriesRepo::ids(&db).await.unwrap(), [series.id]);
}

#[rstest]
#[tokio::test]
async fn saving_a_refreshed_series_replaces_its_seasons_and_episodes(#[future(awt)] db: Database) {
    let mut series =
        Series::add(series_metadata(1, &[(1, &[None, None]), (2, &[None])]), MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

    series.refresh(series_metadata(1, &[(1, &[None])]), now() + 1.hour());
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), Some(series));
}

#[rstest]
#[tokio::test]
async fn save_and_remove_append_their_events(#[future(awt)] db: Database) {
    let mut series = Series::add(series_metadata(1, &[(1, &[None])]), MonitorPreset::All, TODAY, now());
    let added = Event::SeriesAdded { series: series.id, title: series.title.clone() };

    SeriesRepo::save(&db, &mut series, std::slice::from_ref(&added)).await.unwrap();
    SeriesRepo::remove(&db, series.id, std::slice::from_ref(&added)).await.unwrap();

    let events = db.event_log().read_after(None, 10).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), None);
    assert!(SeriesRepo::ids(&db).await.unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn rejects_a_second_series_with_the_same_source(#[future(awt)] db: Database) {
    let mut first = Series::add(series_metadata(1, &[]), MonitorPreset::All, TODAY, now());
    let mut second = Series::add(series_metadata(1, &[]), MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&db, &mut first, &[]).await.unwrap();

    assert!(SeriesRepo::save(&db, &mut second, &[]).await.is_err());
}

#[rstest]
#[tokio::test]
async fn failed_save_writes_no_events(#[future(awt)] db: Database) {
    let mut first = Series::add(series_metadata(1, &[]), MonitorPreset::All, TODAY, now());
    let mut duplicate = Series::add(series_metadata(1, &[]), MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&db, &mut first, &[]).await.unwrap();

    let event = Event::SeriesAdded { series: duplicate.id, title: duplicate.title.clone() };
    let _ = SeriesRepo::save(&db, &mut duplicate, &[event]).await;

    assert!(db.event_log().read_after(None, 10).await.unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn movies_round_trip_and_update(#[future(awt)] db: Database) {
    let mut movie = Movie::add(movie_metadata(438631), true, now());
    MovieRepo::save(&db, &mut movie, &[]).await.unwrap();
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap(), Some(movie.clone()));

    movie.file = Some(MediaFileId::generate());
    movie.refresh(MovieMetadata { title: "Dune: Part One".into(), ..movie_metadata(438631) }, now() + 1.hour());
    MovieRepo::save(&db, &mut movie, &[]).await.unwrap();

    assert_eq!(MovieRepo::find_by_source(&db, ExternalId::Tmdb(438631)).await.unwrap(), Some(movie.clone()));
    assert_eq!(MovieRepo::ids(&db).await.unwrap(), [movie.id]);

    MovieRepo::remove(&db, movie.id, &[]).await.unwrap();
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap(), None);
}

fn any_date() -> impl Strategy<Value = Date> {
    (0..3_650i64).prop_map(|days| date(2020, 1, 1) + days.days())
}

fn any_metadata() -> impl Strategy<Value = SeriesMetadata> {
    prop::collection::btree_map(0..5u16, prop::collection::vec(prop::option::of(any_date()), 0..5), 0..4).prop_map(
        |seasons: BTreeMap<u16, Vec<Option<Date>>>| {
            let seasons: Vec<(u16, &[Option<Date>])> =
                seasons.iter().map(|(&number, dates)| (number, dates.as_slice())).collect();
            series_metadata(1, &seasons)
        },
    )
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn stored_series_matches_memory_after_any_refresh(before in any_metadata(), after in any_metadata()) {
        let (stored, expected) = block_on(async {
            let db = Database::open_in_memory().await.unwrap();
            let mut series = Series::add(before, MonitorPreset::All, TODAY, now());
            SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

            series.refresh(after, now() + 1.hour());
            SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

            (SeriesRepo::get(&db, series.id).await.unwrap(), series)
        });

        prop_assert_eq!(stored, Some(expected));
    }
}

#[rstest]
#[tokio::test]
async fn every_save_bumps_the_revision(#[future] db: Database) {
    let db = db.await;
    let mut series = Series::add(series_metadata(1, &[(1, &[None])]), MonitorPreset::All, TODAY, now());
    let mut movie = Movie::add(movie_metadata(2), true, now());

    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();
    MovieRepo::save(&db, &mut movie, &[]).await.unwrap();

    assert_eq!((series.revision, movie.revision), (2, 1));
    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap().unwrap().revision, 2);
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap().unwrap().revision, 1);
}

#[rstest]
#[tokio::test]
async fn a_save_from_an_older_revision_changes_nothing(#[future] db: Database) {
    let db = db.await;
    let mut series = Series::add(series_metadata(1, &[(1, &[None])]), MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();
    let mut stale = series.clone();
    series.monitored = false;
    SeriesRepo::save(&db, &mut series, &[]).await.unwrap();

    stale.title = "Stale".into();
    let event = Event::SeriesAdded { series: series.id, title: "Stale".into() };
    let error = SeriesRepo::save(&db, &mut stale, &[event]).await.unwrap_err();

    assert!(matches!(error, StorageError::Conflict), "{error}");
    assert_eq!(stale.revision, 1);
    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), Some(series));
    assert!(db.event_log().read_after(None, 10).await.unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn a_removed_item_is_not_saved_back(#[future] db: Database) {
    let db = db.await;
    let mut movie = Movie::add(movie_metadata(2), true, now());
    MovieRepo::save(&db, &mut movie, &[]).await.unwrap();
    MovieRepo::remove(&db, movie.id, &[]).await.unwrap();

    let error = MovieRepo::save(&db, &mut movie, &[]).await.unwrap_err();

    assert!(matches!(error, StorageError::Conflict), "{error}");
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap(), None);
}
