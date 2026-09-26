mod common;

use common::{App, TODAY, movie_metadata, series_metadata};
use jiff::ToSpan;
use rstest::{fixture, rstest};
use yokoku_domain::{EpisodeRef, ExternalId, MonitorPreset, Releases, SourceStatus};
use yokoku_events::Event;
use yokoku_library::{ItemId, LibraryError, ports::MetadataError};

#[fixture]
async fn app() -> App {
    App::new().await
}

#[rstest]
#[tokio::test]
async fn add_series_applies_the_preset_and_records_the_addition(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(
        1,
        "Frieren",
        SourceStatus::Returning,
        &[(1, &[Some(TODAY - 7.days()), Some(TODAY + 7.days())])],
    ));

    let series = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::Future).await.unwrap();

    let monitored: Vec<_> = series.monitored_episodes().map(|(reference, _)| reference).collect();
    assert_eq!(monitored, [EpisodeRef { season: 1, episode: 2 }]);
    assert_eq!(app.library.series(series.id).await.unwrap(), series);
    assert_eq!(app.events().await, [Event::SeriesAdded { series: series.id, title: "Frieren".into() }]);
}

#[rstest]
#[tokio::test]
async fn adding_an_item_twice_is_rejected(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All).await.unwrap();

    let error = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All).await.unwrap_err();

    assert!(matches!(error, LibraryError::AlreadyInLibrary(ExternalId::Tmdb(1))));
    assert_eq!(app.events().await.len(), 1);
}

#[rstest]
#[tokio::test]
async fn adding_an_unknown_item_stores_nothing(#[future(awt)] app: App) {
    let error = app.sync.add_movie(ExternalId::Tmdb(404), true).await.unwrap_err();

    assert!(matches!(error, LibraryError::Metadata(MetadataError::NotFound(ExternalId::Tmdb(404)))));
    assert!(app.library.find_movie(ExternalId::Tmdb(404)).await.unwrap().is_none());
    assert!(app.events().await.is_empty());
}

#[rstest]
#[tokio::test]
async fn add_movie_stores_it_and_records_the_addition(#[future(awt)] app: App) {
    app.metadata.put_movie(movie_metadata(438631, "Dune", Releases { cinema: Some(TODAY), ..Releases::default() }));

    let movie = app.sync.add_movie(ExternalId::Tmdb(438631), false).await.unwrap();

    assert!(!movie.monitored);
    assert_eq!(app.library.movie(movie.id).await.unwrap(), movie);
    assert_eq!(app.events().await, [Event::MovieAdded { movie: movie.id, title: "Dune".into() }]);
}

#[rstest]
#[tokio::test]
async fn search_marks_items_already_in_the_library(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(1, "Dune: Prophecy", SourceStatus::Returning, &[]));
    app.metadata.put_movie(movie_metadata(438631, "Dune", Releases::default()));
    app.sync.add_movie(ExternalId::Tmdb(438631), true).await.unwrap();

    let hits = app.sync.search("dune").await.unwrap();

    let marked: Vec<_> = hits.iter().map(|hit| (hit.result.title.as_str(), hit.in_library)).collect();
    assert_eq!(marked, [("Dune", true), ("Dune: Prophecy", false)]);
}

#[rstest]
#[tokio::test]
async fn refresh_series_stores_new_episodes_and_keeps_changes(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    let series = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All).await.unwrap();
    let first = EpisodeRef { season: 1, episode: 1 };
    app.library.set_episode_monitored(series.id, first, false).await.unwrap();
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None, None])]));

    app.sync.refresh_series(series.id).await.unwrap();

    let stored = app.library.series(series.id).await.unwrap();
    let monitored: Vec<_> = stored.monitored_episodes().map(|(reference, _)| reference).collect();
    assert_eq!(monitored, [EpisodeRef { season: 1, episode: 2 }]);
}

#[rstest]
#[tokio::test]
async fn refresh_all_continues_past_failures(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    app.metadata.put_series(series_metadata(2, "Pluto", SourceStatus::Ended, &[]));
    app.metadata.put_movie(movie_metadata(438631, "Dune", Releases::default()));
    app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All).await.unwrap();
    let gone = app.sync.add_series(ExternalId::Tmdb(2), MonitorPreset::All).await.unwrap();
    app.sync.add_movie(ExternalId::Tmdb(438631), true).await.unwrap();
    app.metadata.forget(ExternalId::Tmdb(2));

    let report = app.sync.refresh_all().await.unwrap();

    assert_eq!(report.refreshed, 2);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].item, ItemId::Series(gone.id));
    assert!(matches!(report.failures[0].error, LibraryError::Metadata(MetadataError::NotFound(_))));
}
