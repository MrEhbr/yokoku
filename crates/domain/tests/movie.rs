use jiff::{
    Timestamp, ToSpan,
    civil::{Date, date},
};
use rstest::rstest;
use yokoku_domain::{ExternalId, FileStatus, Movie, MovieMetadata, MovieStatus, ReleaseKind, Releases};

const TODAY: Date = date(2026, 9, 26);

fn movie(cinema: Option<i64>, digital: Option<i64>, physical: Option<i64>) -> Movie {
    let from_today = |days: Option<i64>| days.map(|days| TODAY + days.days());
    let metadata = MovieMetadata {
        source: ExternalId::Tmdb(438631),
        title: "Dune".into(),
        original_title: "Dune".into(),
        year: Some(2021),
        poster_path: None,
        releases: Releases { cinema: from_today(cinema), digital: from_today(digital), physical: from_today(physical) },
    };
    Movie::add(metadata, true, Timestamp::UNIX_EPOCH)
}

#[rstest]
#[case::no_dates(None, None, None, MovieStatus::Announced)]
#[case::cinema_tomorrow(Some(1), None, None, MovieStatus::Announced)]
#[case::cinema_today(Some(0), Some(30), None, MovieStatus::InCinemas)]
#[case::digital_today(Some(-30), Some(0), None, MovieStatus::Released)]
#[case::physical_only(None, None, Some(-1), MovieStatus::Released)]
#[case::digital_without_cinema(None, Some(-1), None, MovieStatus::Released)]
fn status_follows_release_dates(
    #[case] cinema: Option<i64>,
    #[case] digital: Option<i64>,
    #[case] physical: Option<i64>,
    #[case] expected: MovieStatus,
) {
    assert_eq!(movie(cinema, digital, physical).status(TODAY), expected);
}

#[rstest]
#[case::in_cinemas(Some(-1), None, false, FileStatus::Upcoming)]
#[case::released(Some(-30), Some(-1), false, FileStatus::Missing)]
#[case::downloaded(Some(-30), Some(-1), true, FileStatus::Downloaded)]
fn file_status_is_missing_only_after_home_release(
    #[case] cinema: Option<i64>,
    #[case] digital: Option<i64>,
    #[case] has_file: bool,
    #[case] expected: FileStatus,
) {
    let mut movie = movie(cinema, digital, None);
    movie.has_file = has_file;

    assert_eq!(movie.file_status(TODAY), expected);
}

#[test]
fn refresh_updates_metadata_and_keeps_identity() {
    let mut movie = movie(None, None, None);
    let id = movie.id;
    let later = Timestamp::UNIX_EPOCH + 1.hour();

    movie.refresh(
        MovieMetadata {
            source: movie.source,
            title: "Dune: Part One".into(),
            original_title: "Dune".into(),
            year: Some(2021),
            poster_path: Some("/poster.jpg".into()),
            releases: Releases { cinema: Some(TODAY), ..Releases::default() },
        },
        later,
    );

    assert_eq!(movie.id, id);
    assert_eq!(movie.title, "Dune: Part One");
    assert_eq!(movie.status(TODAY), MovieStatus::InCinemas);
    assert_eq!(movie.refreshed_at, later);
}

#[test]
fn release_dates_are_listed_in_kind_order() {
    let movie = movie(Some(-30), None, Some(10));

    let dates: Vec<_> = movie.releases.dates().collect();

    assert_eq!(dates, [(ReleaseKind::Cinema, TODAY - 30.days()), (ReleaseKind::Physical, TODAY + 10.days())]);
}
