mod common;

use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use jiff::{SignedDuration, ToSpan};
use rstest::{fixture, rstest};
use yokoku_domain::{EpisodeRef, ExternalId, ItemFolder, ItemId, MonitorPreset, Releases, SourceStatus};
use yokoku_events::{MovieAdded, SeriesAdded};
use yokoku_library::{LibraryError, ports::MetadataError};

#[fixture]
async fn app() -> App {
    App::new().await
}

#[rstest]
#[tokio::test]
async fn add_series_applies_the_preset_and_records_the_addition(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(
        1,
        "Frieren",
        SourceStatus::Returning,
        &[(1, &[Some(TODAY - 7.days()), Some(TODAY + 7.days())])],
    ));

    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::Future, ROOT.into(), None).await.unwrap();

    let monitored: Vec<_> = series.monitored_episodes().map(|(reference, _)| reference).collect();
    assert_eq!(monitored, [EpisodeRef { season: 1, episode: 2 }]);
    assert_eq!(app.library.series(series.id).await.unwrap(), series);
    assert_eq!(app.events().await, [SeriesAdded { series: series.id, title: "Frieren".into() }.into()]);
}

#[rstest]
#[tokio::test]
async fn adding_an_item_twice_is_rejected(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();

    let error = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap_err();

    assert!(matches!(error, LibraryError::AlreadyInLibrary(ExternalId::Tmdb(1))));
    assert_eq!(app.events().await.len(), 1);
}

#[rstest]
#[tokio::test]
async fn adding_an_unknown_item_stores_nothing(#[future(awt)] app: App) {
    let error = app.metadata.add_movie(ExternalId::Tmdb(404), true, ROOT.into(), None).await.unwrap_err();

    assert!(matches!(error, LibraryError::Metadata(MetadataError::NotFound(ExternalId::Tmdb(404)))));
    assert!(app.library.find_movie(ExternalId::Tmdb(404)).await.unwrap().is_none());
    assert!(app.events().await.is_empty());
}

#[rstest]
#[tokio::test]
async fn add_movie_stores_it_and_records_the_addition(#[future(awt)] app: App) {
    app.provider.put_movie(movie_metadata(438631, "Dune", Releases { cinema: Some(TODAY), ..Releases::default() }));

    let movie = app.metadata.add_movie(ExternalId::Tmdb(438631), false, ROOT.into(), None).await.unwrap();

    assert!(!movie.monitored);
    assert_eq!(app.library.movie(movie.id).await.unwrap(), movie);
    assert_eq!(app.events().await, [MovieAdded { movie: movie.id, title: "Dune".into() }.into()]);
}

#[rstest]
#[tokio::test]
async fn items_get_the_named_folder_or_the_given_one(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    app.provider.put_movie(movie_metadata(438631, "Dune", Releases::default()));

    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let movie = app
        .metadata
        .add_movie(ExternalId::Tmdb(438631), true, "/films".into(), Some("Dune 2021".into()))
        .await
        .unwrap();

    assert_eq!(series.folder, ItemFolder { root: ROOT.into(), name: "tmdb:1".into() });
    assert_eq!(movie.folder, ItemFolder { root: "/films".into(), name: "Dune 2021".into() });
}

#[rstest]
#[case::empty("")]
#[case::nested("Frieren/Season 1")]
#[case::parent("..")]
#[tokio::test]
async fn a_folder_name_must_be_one_path_component(#[future(awt)] app: App, #[case] folder: &str) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));

    let error = app
        .metadata
        .add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), Some(folder.into()))
        .await
        .unwrap_err();

    assert!(matches!(error, LibraryError::InvalidFolder(_)), "{error}");
    assert!(app.events().await.is_empty());
}

#[rstest]
#[tokio::test]
async fn two_items_cannot_share_a_folder(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    app.provider.put_series(series_metadata(2, "Frieren", SourceStatus::Returning, &[]));
    let folder = || Some("Frieren".to_owned());
    app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), folder()).await.unwrap();

    let error =
        app.metadata.add_series(ExternalId::Tmdb(2), MonitorPreset::All, ROOT.into(), folder()).await.unwrap_err();

    assert!(matches!(error, LibraryError::FolderTaken(_)), "{error}");
    assert_eq!(app.events().await.len(), 1);
}

#[rstest]
#[tokio::test]
async fn refreshing_keeps_the_folder(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.provider.put_series(series_metadata(1, "Sousou no Frieren", SourceStatus::Returning, &[]));

    let refreshed = app.metadata.refresh_series(series.id).await.unwrap();

    assert_eq!(refreshed.title, "Sousou no Frieren");
    assert_eq!(refreshed.folder, series.folder);
}

#[rstest]
#[tokio::test]
async fn search_marks_items_already_in_the_library(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Dune: Prophecy", SourceStatus::Returning, &[]));
    app.provider.put_movie(movie_metadata(438631, "Dune", Releases::default()));
    app.metadata.add_movie(ExternalId::Tmdb(438631), true, ROOT.into(), None).await.unwrap();

    let hits = app.metadata.search("dune").await.unwrap();

    let marked: Vec<_> = hits.iter().map(|hit| (hit.result.title.as_str(), hit.in_library)).collect();
    assert_eq!(marked, [("Dune", true), ("Dune: Prophecy", false)]);
}

#[rstest]
#[tokio::test]
async fn refresh_series_stores_new_episodes_and_keeps_changes(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let first = EpisodeRef { season: 1, episode: 1 };
    app.library.set_episode_monitored(series.id, first, false).await.unwrap();
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None, None])]));

    app.metadata.refresh_series(series.id).await.unwrap();

    let stored = app.library.series(series.id).await.unwrap();
    let monitored: Vec<_> = stored.monitored_episodes().map(|(reference, _)| reference).collect();
    assert_eq!(monitored, [EpisodeRef { season: 1, episode: 2 }]);
}

#[rstest]
#[tokio::test]
async fn refresh_all_continues_past_failures(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));
    app.provider.put_series(series_metadata(2, "Pluto", SourceStatus::Ended, &[]));
    app.provider.put_movie(movie_metadata(438631, "Dune", Releases::default()));
    app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let gone = app.metadata.add_series(ExternalId::Tmdb(2), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.metadata.add_movie(ExternalId::Tmdb(438631), true, ROOT.into(), None).await.unwrap();
    app.provider.forget(ExternalId::Tmdb(2));

    let report = app.metadata.refresh_all().await.unwrap();

    assert_eq!(report.refreshed, 2);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].item, ItemId::Series(gone.id));
    assert!(matches!(report.failures[0].error, LibraryError::Metadata(MetadataError::NotFound(_))));
}

#[rstest]
#[tokio::test]
async fn refresh_due_refreshes_only_what_the_rules_pick(#[future(awt)] app: App) {
    let long_ago = Some(TODAY - 100.days());
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[long_ago])]));
    app.provider.put_series(series_metadata(2, "Pluto", SourceStatus::Ended, &[(1, &[long_ago])]));
    app.provider.put_movie(movie_metadata(438631, "Dune", Releases::default()));
    let running = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.metadata.add_series(ExternalId::Tmdb(2), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let announced = app.metadata.add_movie(ExternalId::Tmdb(438631), true, ROOT.into(), None).await.unwrap();

    let fresh = app.metadata.refresh_due().await.unwrap();
    app.clock.advance(SignedDuration::from_hours(13));
    let due = app.metadata.refresh_due().await.unwrap();

    assert_eq!((fresh.refreshed, fresh.failures.len()), (0, 0));
    assert_eq!(due.refreshed, 2);
    assert!(app.library.series(running.id).await.unwrap().refreshed_at > running.refreshed_at);
    assert!(app.library.movie(announced.id).await.unwrap().refreshed_at > announced.refreshed_at);
}
