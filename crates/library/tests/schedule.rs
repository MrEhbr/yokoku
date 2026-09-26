mod common;

use common::{App, ROOT, TODAY, movie_metadata, series_metadata};
use jiff::{
    ToSpan,
    civil::{Date, date},
};
use rstest::rstest;
use yokoku_domain::{EpisodeRef, ExternalId, FileStatus, MediaFileId, MonitorPreset, Releases, SourceStatus};
use yokoku_library::{CalendarEntry, CalendarRelease, month_of, ports::SeriesRepo, week_of};

/// - Frieren: special tomorrow; S01 at -14 (downloaded), -7, today, +7
/// - Pluto: season 1 unmonitored, episodes at -7 and +3
/// - Dune (monitored): cinema -90, digital -1, physical +5
/// - Arrakis (unmonitored): cinema +2
async fn populated() -> App {
    let app = App::new().await;
    let day = |offset: i64| Some(TODAY + offset.days());
    app.metadata.put_series(series_metadata(
        1,
        "Frieren",
        SourceStatus::Returning,
        &[(0, &[day(1)]), (1, &[day(-14), day(-7), day(0), day(7)])],
    ));
    app.metadata.put_series(series_metadata(2, "Pluto", SourceStatus::Returning, &[(1, &[day(-7), day(3)])]));
    app.metadata.put_movie(movie_metadata(
        10,
        "Dune",
        Releases { cinema: day(-90), digital: day(-1), physical: day(5) },
    ));
    app.metadata.put_movie(movie_metadata(11, "Arrakis", Releases { cinema: day(2), ..Releases::default() }));

    let frieren = app.sync.add_series(ExternalId::Tmdb(1), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    let pluto = app.sync.add_series(ExternalId::Tmdb(2), MonitorPreset::All, ROOT.into(), None).await.unwrap();
    app.sync.add_movie(ExternalId::Tmdb(10), true, ROOT.into(), None).await.unwrap();
    app.sync.add_movie(ExternalId::Tmdb(11), false, ROOT.into(), None).await.unwrap();

    app.library.set_season_monitored(pluto.id, 1, false).await.unwrap();
    let mut frieren = app.library.series(frieren.id).await.unwrap();
    frieren.episode_mut(EpisodeRef { season: 1, episode: 1 }).unwrap().file = Some(MediaFileId::generate());
    SeriesRepo::save(&app.db, &mut frieren, &[]).await.unwrap();
    app
}

fn summary(entries: &[CalendarEntry]) -> Vec<(Date, String, String, FileStatus)> {
    entries
        .iter()
        .map(|entry| {
            let release = match &entry.release {
                CalendarRelease::Episode { reference, .. } => reference.to_string(),
                CalendarRelease::Movie(kind) => format!("{kind:?}"),
            };
            (entry.date, entry.title.clone(), release, entry.status)
        })
        .collect()
}

fn row(offset: i64, title: &str, release: &str, status: FileStatus) -> (Date, String, String, FileStatus) {
    (TODAY + offset.days(), title.into(), release.into(), status)
}

#[tokio::test]
async fn calendar_lists_monitored_releases_in_range_by_date() {
    let app = populated().await;

    let entries = app.schedule.calendar(TODAY - 7.days(), TODAY + 7.days()).await.unwrap();

    assert_eq!(
        summary(&entries),
        [
            row(-7, "Frieren", "S01E02", FileStatus::Missing),
            row(-1, "Dune", "Digital", FileStatus::Missing),
            row(0, "Frieren", "S01E03", FileStatus::Upcoming),
            row(5, "Dune", "Physical", FileStatus::Missing),
            row(7, "Frieren", "S01E04", FileStatus::Upcoming),
        ]
    );
}

#[tokio::test]
async fn calendar_includes_both_ends_of_the_range() {
    let app = populated().await;

    let entries = app.schedule.calendar(TODAY, TODAY).await.unwrap();

    assert_eq!(summary(&entries), [row(0, "Frieren", "S01E03", FileStatus::Upcoming)]);
}

#[tokio::test]
async fn calendar_marks_downloaded_episodes() {
    let app = populated().await;

    let entries = app.schedule.calendar(TODAY - 14.days(), TODAY - 14.days()).await.unwrap();

    assert_eq!(summary(&entries), [row(-14, "Frieren", "S01E01", FileStatus::Downloaded)]);
}

#[tokio::test]
async fn upcoming_starts_today() {
    let app = populated().await;

    let entries = app.schedule.upcoming(7).await.unwrap();

    assert_eq!(
        summary(&entries),
        [
            row(0, "Frieren", "S01E03", FileStatus::Upcoming),
            row(5, "Dune", "Physical", FileStatus::Missing),
            row(7, "Frieren", "S01E04", FileStatus::Upcoming),
        ]
    );
}

#[tokio::test]
async fn missing_groups_aired_monitored_episodes_by_series() {
    let app = populated().await;

    let missing = app.schedule.missing().await.unwrap();

    assert_eq!(missing.series.len(), 1);
    assert_eq!(missing.series[0].title, "Frieren");
    let episodes: Vec<_> = missing.series[0].episodes.iter().map(|e| (e.reference.to_string(), e.air_date)).collect();
    assert_eq!(episodes, [("S01E02".to_owned(), TODAY - 7.days())]);
    let movies: Vec<_> = missing.movies.iter().map(|movie| movie.title.as_str()).collect();
    assert_eq!(movies, ["Dune"]);
}

#[tokio::test]
async fn missing_is_empty_for_an_empty_library() {
    let app = App::new().await;

    let missing = app.schedule.missing().await.unwrap();

    assert!(missing.series.is_empty() && missing.movies.is_empty());
}

#[rstest]
#[case::saturday(date(2026, 9, 26), date(2026, 9, 21), date(2026, 9, 27))]
#[case::monday(date(2026, 9, 21), date(2026, 9, 21), date(2026, 9, 27))]
#[case::sunday(date(2026, 9, 27), date(2026, 9, 21), date(2026, 9, 27))]
#[case::across_months(date(2026, 10, 1), date(2026, 9, 28), date(2026, 10, 4))]
fn weeks_run_monday_to_sunday(#[case] day: Date, #[case] monday: Date, #[case] sunday: Date) {
    assert_eq!(week_of(day), (monday, sunday));
}

#[rstest]
#[case::september(date(2026, 9, 26), date(2026, 9, 1), date(2026, 9, 30))]
#[case::february(date(2026, 2, 10), date(2026, 2, 1), date(2026, 2, 28))]
#[case::leap_february(date(2028, 2, 10), date(2028, 2, 1), date(2028, 2, 29))]
fn months_run_first_to_last_day(#[case] day: Date, #[case] first: Date, #[case] last: Date) {
    assert_eq!(month_of(day), (first, last));
}
