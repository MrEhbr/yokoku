use rstest::rstest;
use yokoku_domain::SubtitleTags;

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
