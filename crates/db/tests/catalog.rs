use std::collections::BTreeMap;

use jiff::{
    Timestamp, ToSpan,
    civil::{Date, date},
    tz::TimeZone,
};
use proptest::prelude::*;
use rstest::{fixture, rstest};
use yokoku_db::Database;
use yokoku_domain::{
    Artwork, EpisodeMetadata, ExternalId, ItemFolder, MediaFileId, MonitorPreset, Movie, MovieId, MovieMetadata,
    Numbering, Releases, SeasonMetadata, Series, SeriesMetadata, SourceStatus, StorageError,
};
use yokoku_library::ports::{MovieRepo, SeriesRepo};
use yokoku_media::ports::Catalog;

const TODAY: Date = date(2026, 9, 26);

#[fixture]
async fn db() -> Database {
    Database::open_in_memory().await.unwrap()
}

fn now() -> Timestamp {
    "2026-09-26T12:00:00.123456789Z".parse().unwrap()
}

fn folder(name: &str) -> ItemFolder {
    ItemFolder::new("/library/tv".into(), name.into()).unwrap()
}

/// Seasons as `(number, air dates)`; source ids are assigned in order, so two calls share ids.
fn series_metadata(source: u64, seasons: &[(u16, &[Option<Date>])]) -> SeriesMetadata {
    let mut next_source_id = 1000;
    SeriesMetadata {
        source: ExternalId::Tmdb(source),
        title: "Frieren".into(),
        original_title: "Sousou no Frieren".into(),
        alternate_titles: vec!["Frieren: Beyond Journey's End".into(), "葬送のフリーレン".into()],
        year: Some(2023),
        artwork: Artwork {
            poster: Some("/frieren.jpg".into()),
            backdrop: Some("/frieren-backdrop.jpg".into()),
            logo: Some("https://artworks.thetvdb.com/banners/v4/series/424536/clearlogo/1.png".into()),
        },
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
        alternate_titles: vec!["Dune: Part One".into()],
        year: None,
        artwork: Artwork::default(),
        releases: Releases { cinema: Some(TODAY), digital: None, physical: Some(TODAY + 90.days()) },
    }
}

#[rstest]
#[tokio::test]
async fn saved_series_loads_back_equal(#[future(awt)] db: Database) {
    let mut series = Series::add(
        series_metadata(1, &[(0, &[None]), (1, &[Some(TODAY), None])]),
        folder("Frieren (2023)"),
        MonitorPreset::Future,
        TODAY,
        now(),
    );
    series.numbering = Numbering::Absolute;
    series.seasons[1].episodes[0].file = Some(MediaFileId::generate());

    SeriesRepo::save(&db, &mut series).await.unwrap();

    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), Some(series.clone()));
    assert_eq!(SeriesRepo::find_by_source(&db, ExternalId::Tmdb(1)).await.unwrap(), Some(series.clone()));
    assert_eq!(SeriesRepo::find_by_source(&db, ExternalId::Tvdb(1)).await.unwrap(), None);
    assert_eq!(SeriesRepo::ids(&db).await.unwrap(), [series.id]);
}

#[rstest]
#[tokio::test]
async fn all_items_load_as_each_one_does(#[future(awt)] db: Database) {
    let metadata = [
        series_metadata(1, &[(0, &[None]), (1, &[Some(TODAY), None])]),
        series_metadata(2, &[]),
        series_metadata(3, &[(1, &[None]), (2, &[None, None, None])]),
    ];
    for (number, metadata) in (1..).zip(metadata) {
        let mut series = Series::add(metadata, folder(&format!("{number}")), MonitorPreset::All, TODAY, now());
        if let Some(season) = series.seasons.last_mut() {
            season.episodes[0].file = Some(MediaFileId::generate());
        }
        SeriesRepo::save(&db, &mut series).await.unwrap();
    }
    for source in [10, 11] {
        let mut movie = Movie::add(movie_metadata(source), folder(&format!("{source}")), true, now());
        MovieRepo::save(&db, &mut movie).await.unwrap();
    }

    let mut each_series = Vec::new();
    for id in SeriesRepo::ids(&db).await.unwrap() {
        each_series.extend(SeriesRepo::get(&db, id).await.unwrap());
    }
    let mut each_movie = Vec::new();
    for id in MovieRepo::ids(&db).await.unwrap() {
        each_movie.extend(MovieRepo::get(&db, id).await.unwrap());
    }
    assert_eq!(each_series.len(), 3);
    assert_eq!(SeriesRepo::all(&db).await.unwrap(), each_series);
    assert_eq!(MovieRepo::all(&db).await.unwrap(), each_movie);
}

#[rstest]
#[tokio::test]
async fn saving_a_refreshed_series_replaces_its_seasons_and_episodes(#[future(awt)] db: Database) {
    let mut series = Series::add(
        series_metadata(1, &[(1, &[None, None]), (2, &[None])]),
        ItemFolder::default(),
        MonitorPreset::All,
        TODAY,
        now(),
    );
    SeriesRepo::save(&db, &mut series).await.unwrap();

    series.refresh(series_metadata(1, &[(1, &[None])]), now() + 1.hour());
    SeriesRepo::save(&db, &mut series).await.unwrap();

    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), Some(series));
}

#[rstest]
#[tokio::test]
async fn removing_a_saved_series_deletes_it(#[future(awt)] db: Database) {
    let mut series =
        Series::add(series_metadata(1, &[(1, &[None])]), ItemFolder::default(), MonitorPreset::All, TODAY, now());

    SeriesRepo::save(&db, &mut series).await.unwrap();
    SeriesRepo::remove(&db, series.id).await.unwrap();

    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), None);
    assert!(SeriesRepo::ids(&db).await.unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn rejects_a_second_series_with_the_same_source(#[future(awt)] db: Database) {
    let mut first = Series::add(series_metadata(1, &[]), folder("first"), MonitorPreset::All, TODAY, now());
    let mut second = Series::add(series_metadata(1, &[]), folder("second"), MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&db, &mut first).await.unwrap();

    assert!(SeriesRepo::save(&db, &mut second).await.is_err());
}

#[rstest]
#[tokio::test]
async fn items_are_found_by_folder_and_cannot_share_one(#[future(awt)] db: Database) {
    let mut series = Series::add(series_metadata(1, &[]), folder("Frieren"), MonitorPreset::All, TODAY, now());
    let mut sharing = Series::add(series_metadata(2, &[]), folder("Frieren"), MonitorPreset::All, TODAY, now());
    let mut movie = Movie::add(movie_metadata(438631), folder("Dune"), true, now());
    SeriesRepo::save(&db, &mut series).await.unwrap();
    MovieRepo::save(&db, &mut movie).await.unwrap();

    assert!(SeriesRepo::save(&db, &mut sharing).await.is_err());
    assert_eq!(SeriesRepo::find_by_folder(&db, &folder("Frieren")).await.unwrap(), Some(series.id));
    assert_eq!(SeriesRepo::find_by_folder(&db, &folder("Dune")).await.unwrap(), None);
    assert_eq!(MovieRepo::find_by_folder(&db, &folder("Dune")).await.unwrap(), Some(movie.id));
}

#[rstest]
#[tokio::test]
async fn movies_round_trip_and_update(#[future(awt)] db: Database) {
    let mut movie = Movie::add(movie_metadata(438631), folder("Dune (2021)"), true, now());
    MovieRepo::save(&db, &mut movie).await.unwrap();
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap(), Some(movie.clone()));

    movie.file = Some(MediaFileId::generate());
    movie.refresh(MovieMetadata { title: "Dune: Part One".into(), ..movie_metadata(438631) }, now() + 1.hour());
    MovieRepo::save(&db, &mut movie).await.unwrap();

    assert_eq!(MovieRepo::find_by_source(&db, ExternalId::Tmdb(438631)).await.unwrap(), Some(movie.clone()));
    assert_eq!(MovieRepo::ids(&db).await.unwrap(), [movie.id]);

    MovieRepo::remove(&db, movie.id).await.unwrap();
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap(), None);
}

fn any_date() -> impl Strategy<Value = Date> {
    (0..3_650i64).prop_map(|days| date(2020, 1, 1) + days.days())
}

fn any_metadata() -> impl Strategy<Value = SeriesMetadata> {
    let seasons = prop::collection::btree_map(0..5u16, prop::collection::vec(prop::option::of(any_date()), 0..5), 0..4);
    let titles = prop::collection::vec("\\PC{0,20}", 0..3);
    (seasons, titles).prop_map(|(seasons, titles): (BTreeMap<u16, Vec<Option<Date>>>, Vec<String>)| {
        let seasons: Vec<(u16, &[Option<Date>])> =
            seasons.iter().map(|(&number, dates)| (number, dates.as_slice())).collect();
        SeriesMetadata { alternate_titles: titles, ..series_metadata(1, &seasons) }
    })
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
            let mut series = Series::add(before, ItemFolder::default(), MonitorPreset::All, TODAY, now());
            SeriesRepo::save(&db, &mut series).await.unwrap();

            series.refresh(after, now() + 1.hour());
            SeriesRepo::save(&db, &mut series).await.unwrap();

            (SeriesRepo::get(&db, series.id).await.unwrap(), series)
        });

        prop_assert_eq!(stored, Some(expected));
    }
}

#[rstest]
#[tokio::test]
async fn every_save_bumps_the_revision(#[future] db: Database) {
    let db = db.await;
    let mut series =
        Series::add(series_metadata(1, &[(1, &[None])]), ItemFolder::default(), MonitorPreset::All, TODAY, now());
    let mut movie = Movie::add(movie_metadata(2), ItemFolder::default(), true, now());

    SeriesRepo::save(&db, &mut series).await.unwrap();
    SeriesRepo::save(&db, &mut series).await.unwrap();
    MovieRepo::save(&db, &mut movie).await.unwrap();

    assert_eq!((series.revision, movie.revision), (2, 1));
    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap().unwrap().revision, 2);
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap().unwrap().revision, 1);
}

#[rstest]
#[tokio::test]
async fn a_save_from_an_older_revision_changes_nothing(#[future] db: Database) {
    let db = db.await;
    let mut series =
        Series::add(series_metadata(1, &[(1, &[None])]), ItemFolder::default(), MonitorPreset::All, TODAY, now());
    SeriesRepo::save(&db, &mut series).await.unwrap();
    let mut stale = series.clone();
    series.monitored = false;
    SeriesRepo::save(&db, &mut series).await.unwrap();

    stale.title = "Stale".into();
    let error = SeriesRepo::save(&db, &mut stale).await.unwrap_err();

    assert!(matches!(error, StorageError::Conflict), "{error}");
    assert_eq!(stale.revision, 1);
    assert_eq!(SeriesRepo::get(&db, series.id).await.unwrap(), Some(series));
}

#[rstest]
#[tokio::test]
async fn a_removed_item_is_not_saved_back(#[future] db: Database) {
    let db = db.await;
    let mut movie = Movie::add(movie_metadata(2), ItemFolder::default(), true, now());
    MovieRepo::save(&db, &mut movie).await.unwrap();
    MovieRepo::remove(&db, movie.id).await.unwrap();

    let error = MovieRepo::save(&db, &mut movie).await.unwrap_err();

    assert!(matches!(error, StorageError::Conflict), "{error}");
    assert_eq!(MovieRepo::get(&db, movie.id).await.unwrap(), None);
}

#[rstest]
#[tokio::test]
async fn the_catalog_reads_the_library(#[future] db: Database) {
    let db = db.await;
    let mut series = Series::add(
        SeriesMetadata {
            source: ExternalId::Tmdb(1),
            title: "Frieren".into(),
            original_title: "Sousou no Frieren".into(),
            alternate_titles: Vec::new(),
            year: Some(2023),
            artwork: Artwork::default(),
            status: SourceStatus::Returning,
            seasons: vec![],
        },
        ItemFolder::default(),
        MonitorPreset::All,
        now().to_zoned(TimeZone::UTC).date(),
        now(),
    );
    let mut movie = Movie::add(
        MovieMetadata {
            source: ExternalId::Tmdb(2),
            title: "Dune".into(),
            original_title: "Dune".into(),
            alternate_titles: Vec::new(),
            year: Some(2021),
            artwork: Artwork::default(),
            releases: Releases::default(),
        },
        ItemFolder::default(),
        true,
        now(),
    );
    SeriesRepo::save(&db, &mut series).await.unwrap();
    MovieRepo::save(&db, &mut movie).await.unwrap();

    assert_eq!(db.all_series().await.unwrap(), std::slice::from_ref(&series));
    assert_eq!(db.all_movies().await.unwrap(), std::slice::from_ref(&movie));
    assert_eq!(Catalog::series(&db, series.id).await.unwrap(), Some(series));
    assert_eq!(Catalog::movie(&db, movie.id).await.unwrap(), Some(movie));
    assert_eq!(Catalog::movie(&db, MovieId::generate()).await.unwrap(), None);
}
