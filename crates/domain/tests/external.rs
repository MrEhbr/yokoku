use proptest::prelude::*;
use rstest::rstest;
use yokoku_domain::ExternalId;

#[rstest]
#[case::tmdb("tmdb:1396", ExternalId::Tmdb(1396))]
#[case::tvdb("tvdb:81189", ExternalId::Tvdb(81189))]
#[case::any_case("TMDB:7", ExternalId::Tmdb(7))]
fn parses_source_ids(#[case] text: &str, #[case] expected: ExternalId) {
    assert_eq!(text.parse::<ExternalId>(), Ok(expected));
}

#[rstest]
#[case::no_provider("1396")]
#[case::unknown_provider("imdb:1396")]
#[case::not_a_number("tmdb:abc")]
#[case::negative("tmdb:-1")]
#[case::empty("")]
fn rejects_malformed_source_ids(#[case] text: &str) {
    assert!(text.parse::<ExternalId>().is_err());
}

fn any_external_id() -> impl Strategy<Value = ExternalId> {
    prop_oneof![any::<u64>().prop_map(ExternalId::Tmdb), any::<u64>().prop_map(ExternalId::Tvdb)]
}

proptest! {
    #[test]
    fn display_and_parse_round_trip(id in any_external_id()) {
        prop_assert_eq!(id.to_string().parse::<ExternalId>(), Ok(id));
    }
}
