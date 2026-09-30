mod common;

use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use jiff::{SignedDuration, ToSpan};
use rstest::{fixture, rstest};
use yokoku_domain::{
    EpisodeMetadata, EpisodeRef, EpisodeSpan, ExternalId, ItemFolder, ItemId, MediaFileId, MediaKind, MonitorPreset,
    Releases, SeasonMetadata, SourceStatus,
};
use yokoku_events::{EpisodesRenumbered, MovieAdded, RenumberedFile, SeriesAdded};
use yokoku_library::{
    LibraryError,
    ports::{MetadataError, SeriesRepo},
};

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

    assert!(matches!(error, LibraryError::AlreadyInLibrary(ExternalId::Tmdb(1))), "{error}");
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

    assert_eq!(series.folder, ItemFolder { root: ROOT.into(), name: "Frieren (2023)".into() });
    assert_eq!(movie.folder, ItemFolder { root: "/films".into(), name: "Dune 2021".into() });
}

#[rstest]
#[tokio::test]
async fn a_folder_name_must_be_one_path_component(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[]));

    let error = app
        .metadata
        .add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), Some("Frieren/Season 1".into()))
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
    let movie = app.metadata.add_movie(ExternalId::Tmdb(438631), true, ROOT.into(), None).await.unwrap();

    let hits = app.metadata.search("dune", None).await.unwrap();

    let marked: Vec<_> = hits.iter().map(|hit| (hit.result.title.as_str(), hit.in_library)).collect();
    assert_eq!(marked, [("Dune", Some(ItemId::Movie(movie.id))), ("Dune: Prophecy", None)]);
}

#[rstest]
#[tokio::test]
async fn a_search_can_ask_for_one_kind(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Dune: Prophecy", SourceStatus::Returning, &[]));
    app.provider.put_movie(movie_metadata(438631, "Dune", Releases::default()));

    let series = app.metadata.search("dune", Some(MediaKind::Series)).await.unwrap();

    let titles: Vec<_> = series.iter().map(|hit| hit.result.title.as_str()).collect();
    assert_eq!(titles, ["Dune: Prophecy"]);
}

#[rstest]
#[tokio::test]
async fn search_hits_carry_the_folder_adding_would_give(#[future(awt)] app: App) {
    let mut frieren = series_metadata(1, "Frieren", SourceStatus::Returning, &[]);
    frieren.description.overview = "An elf mage outlives her party.".into();
    app.provider.put_series(frieren);

    let hit = app.metadata.search("frieren", None).await.unwrap().remove(0);
    let added = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();

    assert_eq!(hit.folder, added.folder.name);
    assert_eq!(hit.result.overview, "An elf mage outlives her party.");
}

#[rstest]
#[tokio::test]
async fn refresh_series_stores_new_episodes(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None, None])]));

    app.metadata.refresh_series(series.id).await.unwrap();

    assert_eq!(app.library.series(series.id).await.unwrap().episodes().count(), 2);
    assert_eq!(app.events().await.len(), 1);
}

#[rstest]
#[tokio::test]
async fn refresh_series_records_files_whose_episodes_were_renumbered(#[future(awt)] app: App) {
    let original = series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None, None])]);
    app.provider.put_series(original.clone());
    let mut series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let file = MediaFileId::generate();
    series.episode_mut(EpisodeRef { season: 1, episode: 2 }).unwrap().file = Some(file);
    SeriesRepo::save(&app.db, &mut series).await.unwrap();
    let mut renumbered = original;
    let episode = renumbered.seasons[0].episodes.pop().unwrap();
    renumbered.seasons.push(SeasonMetadata { number: 2, episodes: vec![EpisodeMetadata { number: 1, ..episode }] });
    app.provider.put_series(renumbered);

    app.metadata.refresh_series(series.id).await.unwrap();

    let renumbered =
        EpisodesRenumbered { series: series.id, files: vec![RenumberedFile { file, span: EpisodeSpan::new(2, 1, 1) }] };
    assert_eq!(app.events().await.last(), Some(&renumbered.into()));
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
    assert!(
        matches!(report.failures[0].error, LibraryError::Metadata(MetadataError::NotFound(_))),
        "{}",
        report.failures[0].error
    );
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
