use proptest::prelude::*;
use rstest::rstest;
use serde_json::json;
use yokoku_domain::{EpisodeRef, EpisodeSpan};

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
