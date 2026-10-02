use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use jiff::{SignedDuration, Timestamp, ToSpan};
use rstest::{fixture, rstest};
use yokoku_core::{
    integrations::ports::{Watched, WatchedStore},
    library::{FileCount, LibraryError, LibraryFilter, LibrarySort, LibraryStatus, WatchState},
    media::{
        MediaFile,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{
    EpisodeRef, EpisodeSpan, ExternalId, FileTarget, MediaFileId, MediaKind, MonitorPreset, MovieStatus, Numbering,
    Releases, SeriesId, SeriesStatus, SourceStatus,
    events::{MovieRemoved, SeriesRemoved},
};

use crate::common;

#[fixture]
async fn app() -> App {
    App::new().await
}

/// Adds, one hour apart: "frieren" (continuing, next episode in 7 days, one file),
/// "Pluto" (ended), "Dune" (released), "Arrakis" (announced for 30 days from today).
async fn populated() -> App {
    let app = App::new().await;
    app.provider.put_series(series_metadata(
        1,
        "frieren",
        SourceStatus::Returning,
        &[(1, &[Some(TODAY - 7.days()), Some(TODAY + 7.days())])],
    ));
    app.provider.put_series(series_metadata(2, "Pluto", SourceStatus::Ended, &[(1, &[Some(TODAY - 700.days())])]));
    app.provider.put_movie(movie_metadata(
        10,
        "Dune",
        Releases {
            cinema: Some(TODAY - 90.days()),
            digital: Some(TODAY - 30.days()),
            physical: Some(TODAY + 60.days()),
        },
    ));
    app.provider.put_movie(movie_metadata(
        11,
        "Arrakis",
        Releases { cinema: Some(TODAY + 30.days()), ..Releases::default() },
    ));

    let frieren = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.clock.advance(SignedDuration::from_hours(1));
    app.metadata.add_series(ExternalId::Tmdb(2), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.clock.advance(SignedDuration::from_hours(1));
    app.metadata.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();
    app.clock.advance(SignedDuration::from_hours(1));
    app.metadata.add_movie(ExternalId::Tmdb(11), true, ROOT.into(), None).await.unwrap();

    let mut frieren = app.library.series(frieren.id).await.unwrap();
    frieren.seasons[0].episodes[0].file = Some(MediaFileId::generate());
    yokoku_core::library::ports::SeriesRepo::save(&app.db, &mut frieren).await.unwrap();
    app
}

#[tokio::test]
async fn list_filters_and_sorts_entries() {
    let app = populated().await;
    let movies = LibraryFilter { kind: Some(MediaKind::Movie), ..LibraryFilter::default() };
    let titles = async |filter, sort| {
        let entries = app.library.list(filter, sort).await.unwrap();
        entries.into_iter().map(|entry| entry.title).collect::<Vec<_>>()
    };

    let by_release = titles(LibraryFilter::default(), LibrarySort::NextRelease).await;
    let movie_titles = titles(movies, LibrarySort::Title).await;

    assert_eq!(by_release, ["frieren", "Arrakis", "Dune", "Pluto"]);
    assert_eq!(movie_titles, ["Arrakis", "Dune"]);
}

#[tokio::test]
async fn list_filters_by_what_the_media_server_user_watched() {
    let app = populated().await;
    let frieren = app.library.find_series(ExternalId::Tmdb(1)).await.unwrap().unwrap();
    let file = frieren.seasons[0].episodes[0].file.unwrap();
    let stored = MediaFile {
        id: file,
        path: "/tv/frieren/S01E01.mkv".into(),
        size: 1,
        target: FileTarget::Episodes { series: frieren.id, span: EpisodeSpan::new(1, 1, 1).unwrap() },
        added_at: Timestamp::UNIX_EPOCH,
    };
    MediaRepo::save(&app.db, &Changes { added_files: vec![stored], ..Changes::default() }).await.unwrap();
    let titles = async |watched| {
        let filter = LibraryFilter { watched: Some(watched), ..LibraryFilter::default() };
        let entries = app.library.list(filter, LibrarySort::Title).await.unwrap();
        entries.into_iter().map(|entry| entry.title).collect::<Vec<_>>()
    };

    let before = (titles(WatchState::Unwatched).await, titles(WatchState::Watched).await);
    app.db.replace_watched(&[Watched { file, at: None }]).await.unwrap();
    let after = (titles(WatchState::Unwatched).await, titles(WatchState::Watched).await);

    assert_eq!(before.0, ["Arrakis", "Dune", "frieren", "Pluto"]);
    assert!(before.1.is_empty());
    assert_eq!(after.0, ["Arrakis", "Dune", "Pluto"]);
    assert_eq!(after.1, ["frieren"]);
}

#[tokio::test]
async fn list_entries_show_status_files_and_next_release() {
    let app = populated().await;

    let entries = app.library.list(LibraryFilter::default(), LibrarySort::Title).await.unwrap();

    let summary: Vec<_> =
        entries.iter().map(|entry| (entry.title.as_str(), entry.status, entry.files, entry.next_release)).collect();
    let files = |downloaded, missing| FileCount { downloaded, missing };
    assert_eq!(
        summary,
        [
            ("Arrakis", LibraryStatus::Movie(MovieStatus::Announced), files(0, 0), Some(TODAY + 30.days())),
            ("Dune", LibraryStatus::Movie(MovieStatus::Released), files(0, 1), Some(TODAY + 60.days())),
            ("frieren", LibraryStatus::Series(SeriesStatus::Continuing), files(1, 0), Some(TODAY + 7.days())),
            ("Pluto", LibraryStatus::Series(SeriesStatus::Ended), files(0, 1), None),
        ]
    );
}

#[rstest]
#[tokio::test]
async fn monitoring_and_numbering_changes_are_stored(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(
        1,
        "Frieren",
        SourceStatus::Returning,
        &[(1, &[None, None]), (2, &[None])],
    ));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();

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
async fn a_season_takes_its_episodes_along_when_monitored(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(
        1,
        "Frieren",
        SourceStatus::Returning,
        &[(1, &[None, None]), (2, &[None])],
    ));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let episodes = async || {
        let stored = app.library.series(series.id).await.unwrap();
        stored
            .seasons
            .iter()
            .map(|season| (season.monitored, season.episodes.iter().map(|e| e.monitored).collect::<Vec<_>>()))
            .collect::<Vec<_>>()
    };

    app.library.set_episode_monitored(series.id, EpisodeRef { season: 1, episode: 2 }, false).await.unwrap();
    app.library.set_season_monitored(series.id, 1, false).await.unwrap();
    let off = episodes().await;
    app.library.set_season_monitored(series.id, 1, true).await.unwrap();
    let on = episodes().await;

    assert_eq!(off, [(false, vec![false, false]), (true, vec![true])]);
    assert_eq!(on, [(true, vec![true, true]), (true, vec![true])]);
}

#[rstest]
#[tokio::test]
async fn monitoring_rejects_unknown_targets(#[future(awt)] app: App) {
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
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
    app.provider.put_series(series_metadata(1, "Frieren", SourceStatus::Returning, &[(1, &[None])]));
    app.provider.put_movie(movie_metadata(10, "Dune", Releases::default()));
    let series = app.metadata.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let movie = app.metadata.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();

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
