use proptest::prelude::*;
use rstest::rstest;
use yokoku_domain::{Confidence, Numbering, SourceStatus};

fn any_source_status() -> impl Strategy<Value = SourceStatus> {
    prop_oneof![
        Just(SourceStatus::Returning),
        Just(SourceStatus::Planned),
        Just(SourceStatus::InProduction),
        Just(SourceStatus::Pilot),
        Just(SourceStatus::Ended),
        Just(SourceStatus::Canceled),
        Just(SourceStatus::Unknown),
    ]
}

fn any_numbering() -> impl Strategy<Value = Numbering> {
    prop_oneof![Just(Numbering::Standard), Just(Numbering::Absolute)]
}

fn any_confidence() -> impl Strategy<Value = Confidence> {
    prop_oneof![Just(Confidence::Unknown), Just(Confidence::Guess), Just(Confidence::Certain)]
}

proptest! {
    #[test]
    fn source_status_round_trips(status in any_source_status()) {
        prop_assert_eq!(status.as_str().parse::<SourceStatus>(), Ok(status));
    }

    #[test]
    fn numbering_round_trips(numbering in any_numbering()) {
        prop_assert_eq!(numbering.as_str().parse::<Numbering>(), Ok(numbering));
    }

    #[test]
    fn confidence_round_trips(confidence in any_confidence()) {
        prop_assert_eq!(confidence.as_str().parse::<Confidence>(), Ok(confidence));
    }
}

#[rstest]
#[case::empty("")]
#[case::other_case("Returning")]
#[case::variant_name("InProduction")]
#[case::padded(" ended")]
#[case::unknown_name("airing")]
fn rejects_names_outside_the_table(#[case] text: &str) {
    assert!(text.parse::<SourceStatus>().is_err());
}

#[test]
fn parse_error_names_the_kind_and_value() {
    let error = "airing".parse::<SourceStatus>().unwrap_err();
    assert_eq!(error.to_string(), r#"unknown source status "airing""#);
}
