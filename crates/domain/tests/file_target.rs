use rstest::rstest;
use uuid::Uuid;
use yokoku_domain::{EpisodeSpan, FileTarget, MovieId, SeriesId};

fn episodes(series: u128, season: u16, first: u16, last: u16) -> FileTarget {
    FileTarget::Episodes {
        series: SeriesId(Uuid::from_u128(series)),
        span: EpisodeSpan::new(season, first, last).unwrap(),
    }
}

fn movie(id: u128) -> FileTarget {
    FileTarget::Movie(MovieId(Uuid::from_u128(id)))
}

#[rstest]
#[case::same_episode(episodes(1, 1, 2, 2), episodes(1, 1, 2, 2), true)]
#[case::ranges_share_an_episode(episodes(1, 1, 1, 3), episodes(1, 1, 3, 4), true)]
#[case::adjacent_ranges(episodes(1, 1, 1, 2), episodes(1, 1, 3, 4), false)]
#[case::other_season(episodes(1, 1, 1, 2), episodes(1, 2, 1, 2), false)]
#[case::other_series(episodes(1, 1, 1, 2), episodes(2, 1, 1, 2), false)]
#[case::same_movie(movie(1), movie(1), true)]
#[case::other_movie(movie(1), movie(2), false)]
#[case::episode_and_movie(episodes(1, 1, 1, 1), movie(1), false)]
fn targets_overlap_when_they_share_an_episode_or_movie(
    #[case] a: FileTarget,
    #[case] b: FileTarget,
    #[case] expected: bool,
) {
    assert_eq!(a.overlaps(&b), expected);
    assert_eq!(b.overlaps(&a), expected);
}
