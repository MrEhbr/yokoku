mod common;

use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use jiff::{SignedDuration, ToSpan};
use rstest::{fixture, rstest};
use yokoku_domain::{
    EpisodeRef, ExternalId, MediaFileId, MediaKind, MonitorPreset, MovieStatus, Numbering, Releases, SeriesId,
    SeriesStatus, SourceStatus,
};
use yokoku_events::{MovieRemoved, SeriesRemoved};
use yokoku_library::{LibraryError, LibraryFilter, LibrarySort, LibraryStatus};

#[fixture]
async fn app() -> App {
    App::new().await
}

/// Adds, one hour apart: "frieren" (continuing, next episode in 7 days, one file),
/// "Pluto" (ended), "Dune" (released), "Arrakis" (announced for 30 days from today).
async fn populated() -> App {
    let app = App::new().await;
    app.metadata.put_series(series_metadata(
        1,
        "frieren",
        SourceStatus::Returning,
        &[(1, &[Some(TODAY - 7.days()), Some(TODAY + 7.days())])],
    ));
    app.metadata.put_series(series_metadata(2, "Pluto", SourceStatus::Ended, &[(1, &[Some(TODAY - 700.days())])]));
    app.metadata.put_movie(movie_metadata(
        10,
        "Dune",
        Releases {
            cinema: Some(TODAY - 90.days()),
            digital: Some(TODAY - 30.days()),
            physical: Some(TODAY + 60.days()),
        },
    ));
    app.metadata.put_movie(movie_metadata(
        11,
        "Arrakis",
        Releases { cinema: Some(TODAY + 30.days()), ..Releases::default() },
    ));

    let frieren = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.clock.advance(SignedDuration::from_hours(1));
    app.sync.add_series(ExternalId::Tmdb(2), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.clock.advance(SignedDuration::from_hours(1));
    app.sync.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();
    app.clock.advance(SignedDuration::from_hours(1));
    app.sync.add_movie(ExternalId::Tmdb(11), true, ROOT.into(), None).await.unwrap();

    let mut frieren = app.library.series(frieren.id).await.unwrap();
    frieren.seasons[0].episodes[0].file = Some(MediaFileId::generate());
    yokoku_library::ports::SeriesRepo::save(&app.db, &mut frieren, &[]).await.unwrap();
    app
}

#[rstest]
#[case::by_title(LibrarySort::Title, &["Arrakis", "Dune", "frieren", "Pluto"])]
#[case::newest_first(LibrarySort::Added, &["Arrakis", "Dune", "Pluto", "frieren"])]
#[case::next_release(LibrarySort::NextRelease, &["frieren", "Arrakis", "Dune", "Pluto"])]
#[tokio::test]
async fn list_sorts_entries(#[case] sort: LibrarySort, #[case] expected: &[&str]) {
    let app = populated().await;

    let entries = app.library.list(LibraryFilter::default(), sort).await.unwrap();

    let titles: Vec<_> = entries.iter().map(|entry| entry.title.as_str()).collect();
    assert_eq!(titles, expected);
}

#[rstest]
#[case::series(LibraryFilter { kind: Some(MediaKind::Series), status: None }, &["frieren", "Pluto"])]
#[case::movies(LibraryFilter { kind: Some(MediaKind::Movie), status: None }, &["Arrakis", "Dune"])]
#[case::ended(LibraryFilter { kind: None, status: Some(LibraryStatus::Series(SeriesStatus::Ended)) }, &["Pluto"])]
#[case::released(LibraryFilter { kind: None, status: Some(LibraryStatus::Movie(MovieStatus::Released)) }, &["Dune"])]
#[tokio::test]
async fn list_filters_entries(#[case] filter: LibraryFilter, #[case] expected: &[&str]) {
    let app = populated().await;

    let entries = app.library.list(filter, LibrarySort::Title).await.unwrap();

    let titles: Vec<_> = entries.iter().map(|entry| entry.title.as_str()).collect();
    assert_eq!(titles, expected);
}

#[tokio::test]
async fn list_entries_show_status_files_and_next_release() {
    let app = populated().await;

    let entries = app.library.list(LibraryFilter::default(), LibrarySort::Title).await.unwrap();

    let summary: Vec<_> =
        entries.iter().map(|entry| (entry.title.as_str(), entry.status, entry.has_files, entry.next_release)).collect();
    assert_eq!(
        summary,
        [
            ("Arrakis", LibraryStatus::Movie(MovieStatus::Announced), false, Some(TODAY + 30.days())),
            ("Dune", LibraryStatus::Movie(MovieStatus::Released), false, Some(TODAY + 60.days())),
            ("frieren", LibraryStatus::Series(SeriesStatus::Continuing), true, Some(TODAY + 7.days())),
            ("Pluto", LibraryStatus::Series(SeriesStatus::Ended), false, None),
        ]
    );
}

#[rstest]
#[tokio::test]
async fn monitoring_and_numbering_changes_are_stored(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(
        1,
        "Frieren",
        SourceStatus::Returning,
        &[(1, &[None, None]), (2, &[None])],
    ));
    let series = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();

    app.library.set_season_monitored(series.id, 2, false).await.unwrap();
    app.library.set_episode_monitored(series.id, EpisodeRef { season: 1, episode: 1 }, false).await.unwrap();
    app.library.set_numbering(series.id, Numbering::Absolute).await.unwrap();

    let stored = app.library.series(series.id).await.unwrap();
    let monitored: Vec<_> = stored.monitored_episodes().map(|(reference, _)| reference).collect();
    assert_eq!(monitored, [EpisodeRef { season: 1, episode: 2 }]);
    assert_eq!(stored.numbering, Numbering::Absolute);

    app.library.set_series_monitored(series.id, false).await.unwrap();
    assert_eq!(app.library.series(series.id).await.unwrap().monitored_episodes().count(), 0);
}

#[rstest]
#[tokio::test]
async fn monitoring_rejects_unknown_targets(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    let series = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let missing_episode = EpisodeRef { season: 1, episode: 9 };
    let unknown = SeriesId::generate();

    assert!(matches!(app.library.set_season_monitored(series.id, 5, true).await, Err(LibraryError::SeasonNotFound(5))));
    assert!(matches!(
        app.library.set_episode_monitored(series.id, missing_episode, true).await,
        Err(LibraryError::EpisodeNotFound(reference)) if reference == missing_episode
    ));
    assert!(matches!(
        app.library.set_series_monitored(unknown, true).await,
        Err(LibraryError::SeriesNotFound(id)) if id == unknown
    ));
}

#[rstest]
#[tokio::test]
async fn removing_items_records_whether_files_go_too(#[future(awt)] app: App) {
    app.metadata.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    app.metadata.put_movie(movie_metadata(10, "Dune", Releases::default()));
    let series = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let movie = app.sync.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();

    app.library.remove_series(series.id, true).await.unwrap();
    app.library.remove_movie(movie.id, false).await.unwrap();

    assert!(app.library.find_series(ExternalId::Tmdb(1)).await.unwrap().is_none());
    assert!(app.library.find_movie(ExternalId::Tmdb(10)).await.unwrap().is_none());
    assert_eq!(
        app.events().await[2..],
        [
            SeriesRemoved { series: series.id, title: "Frieren".into(), delete_files: true }.into(),
            MovieRemoved { movie: movie.id, title: "Dune".into(), delete_files: false }.into(),
        ]
    );
}
