use rstest::rstest;
use serde_json::json;
use yokoku_domain::{TrackerId, TrackerSet, Trackers};

fn id(text: &str) -> TrackerId {
    text.parse().unwrap()
}

#[rstest]
#[case::plain("rutor")]
#[case::dashes_and_digits("rutracker-org_2")]
fn tracker_ids_of_letters_digits_dashes_and_underscores_parse(#[case] text: &str) {
    assert_eq!(text.parse::<TrackerId>().map(|id| id.to_string()), Ok(text.to_owned()));
}

#[rstest]
#[case::empty("")]
#[case::path("../admin")]
#[case::slash("rutor/x")]
#[case::query("rutor?t=caps")]
#[case::space("ru tor")]
#[case::non_ascii("рутор")]
fn other_tracker_ids_are_refused(#[case] text: &str) {
    assert!(text.parse::<TrackerId>().is_err(), "{text:?}");
}

#[test]
fn a_tracker_set_keeps_each_tracker_once_in_order() {
    let set = TrackerSet::new([id("rutor"), id("anilibria"), id("rutor")]).unwrap();

    assert_eq!(set.ids(), [id("rutor"), id("anilibria")]);
}

#[test]
fn a_tracker_set_has_a_tracker() {
    assert_eq!(TrackerSet::new([]), None);
}

#[rstest]
#[case::all(json!("all"), Some(Trackers::All))]
#[case::only(json!({ "only": ["rutor"] }), Some(Trackers::Only(TrackerSet::new([id("rutor")]).unwrap())))]
#[case::only_nothing(json!({ "only": [] }), None)]
#[case::only_a_path(json!({ "only": ["../admin"] }), None)]
fn trackers_deserialize_only_when_valid(#[case] value: serde_json::Value, #[case] expected: Option<Trackers>) {
    assert_eq!(serde_json::from_value::<Trackers>(value).ok(), expected);
}
