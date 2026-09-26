use proptest::prelude::*;
use rstest::rstest;
use yokoku_naming::sanitize;

#[rstest]
#[case::plain("Frieren", "Frieren")]
#[case::subtitle_colon("Dune: Part Two", "Dune - Part Two")]
#[case::bare_colon("12:01", "12-01")]
#[case::separators("AC/DC \\ Live | 1991", "AC-DC - Live - 1991")]
#[case::quotes("The \"Best\" Show", "The 'Best' Show")]
#[case::wildcards("What? <Really> *Now*", "What Really Now")]
#[case::control_characters("Tab\there\nnewline", "Tab here newline")]
#[case::extra_spaces("  Too   many  spaces ", "Too many spaces")]
#[case::trailing_dots("Mr. Robot...", "Mr. Robot")]
#[case::reserved_name("CON", "CON_")]
#[case::reserved_any_case("nul", "nul_")]
#[case::dots_only("..", "_")]
#[case::empty("", "_")]
#[case::cyrillic("Ведьмак", "Ведьмак")]
#[case::japanese("葬送のフリーレン", "葬送のフリーレン")]
fn sanitizes_names(#[case] name: &str, #[case] expected: &str) {
    assert_eq!(sanitize(name), expected);
}

#[test]
fn truncates_on_a_character_boundary() {
    let name = "フ".repeat(200);

    let sanitized = sanitize(&name);

    assert!(sanitized.len() <= 255);
    assert!(sanitized.chars().all(|c| c == 'フ'));
}

proptest! {
    #[test]
    fn sanitized_names_are_always_valid(name in any::<String>()) {
        let sanitized = sanitize(&name);

        prop_assert!(!sanitized.is_empty());
        prop_assert!(sanitized.len() <= 255);
        prop_assert!(!sanitized.chars().any(|c| "/\\:*?\"<>|".contains(c) || c.is_control()));
        prop_assert!(!sanitized.starts_with(' ') && !sanitized.ends_with(' ') && !sanitized.ends_with('.'));
        prop_assert!(!sanitized.contains("  "));
    }

    #[test]
    fn sanitizing_twice_changes_nothing(name in any::<String>()) {
        let once = sanitize(&name);

        prop_assert_eq!(sanitize(&once), once);
    }
}
