use rstest::rstest;
use yokoku_domain::naming::{Naming, NamingTemplates, PatternKind, TemplateError};

fn error(templates: NamingTemplates) -> TemplateError {
    Naming::new(&templates).unwrap_err()
}

#[rstest]
#[case::unknown_token(
    NamingTemplates { movie_file: "{name}".into(), ..NamingTemplates::default() },
    PatternKind::MovieFile, "unknown token {name}"
)]
#[case::token_not_allowed_here(
    NamingTemplates { movie_folder: "{title} {season}".into(), ..NamingTemplates::default() },
    PatternKind::MovieFolder, "{season} cannot be used in this pattern"
)]
#[case::unclosed_token(
    NamingTemplates { series_folder: "{title".into(), ..NamingTemplates::default() },
    PatternKind::SeriesFolder, "unclosed {title"
)]
#[case::stray_brace(
    NamingTemplates { series_folder: "title}".into(), ..NamingTemplates::default() },
    PatternKind::SeriesFolder, "unmatched }"
)]
#[case::unclosed_group(
    NamingTemplates { movie_file: "{title}[ ({year})".into(), ..NamingTemplates::default() },
    PatternKind::MovieFile, "unclosed ["
)]
#[case::stray_bracket(
    NamingTemplates { movie_file: "{title}]".into(), ..NamingTemplates::default() },
    PatternKind::MovieFile, "unmatched ]"
)]
#[case::nested_groups(
    NamingTemplates { movie_file: "{title}[ [{year}]]".into(), ..NamingTemplates::default() },
    PatternKind::MovieFile, "optional groups cannot be nested"
)]
#[case::path_separator(
    NamingTemplates { movie_folder: "Movies/{title}".into(), ..NamingTemplates::default() },
    PatternKind::MovieFolder, "a pattern names one folder or file and cannot contain / or \\"
)]
#[case::episodes_required(
    NamingTemplates { episode_file: "{title} - {episode_title}".into(), ..NamingTemplates::default() },
    PatternKind::EpisodeFile, "must contain {episodes}"
)]
#[case::season_required(
    NamingTemplates { season_folder: "Season".into(), ..NamingTemplates::default() },
    PatternKind::SeasonFolder, "must contain {season}"
)]
#[case::empty(
    NamingTemplates { movie_folder: "".into(), ..NamingTemplates::default() },
    PatternKind::MovieFolder, "must not be empty"
)]
fn invalid_templates_are_rejected(
    #[case] templates: NamingTemplates,
    #[case] pattern: PatternKind,
    #[case] message: &str,
) {
    assert_eq!(error(templates), TemplateError { pattern, message: message.into() });
}

#[test]
fn errors_name_the_pattern() {
    let error = error(NamingTemplates { season_folder: "Season".into(), ..NamingTemplates::default() });

    assert_eq!(error.to_string(), "season folder pattern: must contain {season}");
}

#[test]
fn default_templates_are_valid() {
    assert!(Naming::new(&NamingTemplates::default()).is_ok());
}
