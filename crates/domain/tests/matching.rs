use proptest::prelude::*;
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;
use yokoku_domain::{EpisodeRef, EpisodeSpan, FileTarget, MovieId, SeriesId, SubtitleTags};

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

fn refs(pairs: &[(u16, u16)]) -> Vec<EpisodeRef> {
    pairs.iter().map(|&(season, episode)| EpisodeRef { season, episode }).collect()
}

#[rstest]
#[case(1, 2, "S01E02")]
#[case(0, 13, "S00E13")]
#[case(12, 104, "S12E104")]
fn episode_refs_display_as_sxxexx(#[case] season: u16, #[case] episode: u16, #[case] expected: &str) {
    assert_eq!(EpisodeRef { season, episode }.to_string(), expected);
}

#[rstest]
#[case::single(EpisodeSpan::single(EpisodeRef { season: 1, episode: 2 }), "S01E02", &[(1, 2)])]
#[case::range(EpisodeSpan::new(1, 1, 3).unwrap(), "S01E01-E03", &[(1, 1), (1, 2), (1, 3)])]
#[case::special(EpisodeSpan::new(0, 4, 4).unwrap(), "S00E04", &[(0, 4)])]
fn episode_spans_display_and_list_their_episodes(
    #[case] span: EpisodeSpan,
    #[case] display: &str,
    #[case] expected: &[(u16, u16)],
) {
    assert_eq!(span.to_string(), display);
    assert_eq!(span.refs().collect::<Vec<_>>(), refs(expected));
}

#[test]
fn episode_spans_reject_reversed_ranges() {
    assert_eq!(EpisodeSpan::new(1, 3, 2), None);
}

#[rstest]
#[case::range(json!({ "season": 1, "first": 1, "last": 3 }), EpisodeSpan::new(1, 1, 3))]
#[case::reversed(json!({ "season": 1, "first": 3, "last": 2 }), None)]
fn episode_spans_are_checked_when_deserialized(
    #[case] stored: serde_json::Value,
    #[case] expected: Option<EpisodeSpan>,
) {
    assert_eq!(serde_json::from_value::<EpisodeSpan>(stored).ok(), expected);
}

proptest! {
    #[test]
    fn episode_spans_round_trip_through_json(season: u16, first: u16, length in 0..10u16) {
        let span = EpisodeSpan::new(season, first, first.saturating_add(length)).unwrap();
        let stored = serde_json::to_value(span).unwrap();
        prop_assert_eq!(serde_json::from_value::<EpisodeSpan>(stored).unwrap(), span);
    }
}

#[rstest]
#[case::single("S01E02", EpisodeSpan::new(1, 2, 2))]
#[case::range("S01E01-E03", EpisodeSpan::new(1, 1, 3))]
#[case::lowercase("s00e13", EpisodeSpan::new(0, 13, 13))]
#[case::long_numbers("S12E104", EpisodeSpan::new(12, 104, 104))]
#[case::reversed("S01E03-E01", None)]
#[case::no_episode("S01", None)]
#[case::bare_range("S01E01-03", None)]
#[case::signed("S+1E02", None)]
#[case::empty("", None)]
#[case::overflow("S01E70000", None)]
fn episode_spans_parse_from_their_display_form(#[case] text: &str, #[case] expected: Option<EpisodeSpan>) {
    assert_eq!(text.parse::<EpisodeSpan>().ok(), expected);
}

proptest! {
    #[test]
    fn episode_spans_parse_back_from_display(season: u16, first: u16, length in 0..10u16) {
        let span = EpisodeSpan::new(season, first, first.saturating_add(length)).unwrap();
        prop_assert_eq!(span.to_string().parse::<EpisodeSpan>(), Ok(span));
    }
}

#[rstest]
#[case::plain(Some("en"), false, false, "en")]
#[case::sdh(Some("en"), true, false, "en (SDH)")]
#[case::forced(Some("pt-br"), false, true, "pt-br (forced)")]
#[case::both(Some("en"), true, true, "en (SDH, forced)")]
#[case::no_language(None, false, true, "unknown (forced)")]
fn names_the_language_with_its_flags(
    #[case] language: Option<&str>,
    #[case] sdh: bool,
    #[case] forced: bool,
    #[case] expected: &str,
) {
    let tags = SubtitleTags { language: language.map(Into::into), sdh, forced };

    assert_eq!(tags.to_string(), expected);
}
