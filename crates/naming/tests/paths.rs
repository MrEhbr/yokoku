use std::path::{Path, PathBuf};

use jiff::Timestamp;
use proptest::prelude::*;
use rstest::rstest;
use yokoku_domain::{
    EpisodeMetadata, EpisodeRef, EpisodeSpan, ExternalId, MonitorPreset, Movie, MovieMetadata, Releases,
    SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};
use yokoku_naming::{Naming, NamingError, NamingTemplates, sanitize};

fn movie(title: &str, year: Option<i16>) -> Movie {
    let metadata = MovieMetadata {
        source: ExternalId::Tmdb(1),
        title: title.into(),
        original_title: title.into(),
        alternate_titles: Vec::new(),
        year,
        poster_path: None,
        releases: Releases::default(),
    };
    Movie::add(metadata, true, Timestamp::UNIX_EPOCH)
}

/// Seasons as `(number, episode titles)`.
fn series(title: &str, year: Option<i16>, seasons: &[(u16, &[&str])]) -> Series {
    let mut source_id = 0;
    let metadata = SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: title.into(),
        original_title: title.into(),
        alternate_titles: Vec::new(),
        year,
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: seasons
            .iter()
            .map(|&(number, titles)| SeasonMetadata {
                number,
                episodes: titles
                    .iter()
                    .zip(1..)
                    .map(|(title, episode)| {
                        source_id += 1;
                        EpisodeMetadata { source_id, number: episode, title: (*title).into(), air_date: None }
                    })
                    .collect(),
            })
            .collect(),
    };
    Series::add(metadata, MonitorPreset::All, jiff::civil::date(2026, 1, 1), Timestamp::UNIX_EPOCH)
}

fn span(season: u16, first: u16, last: u16) -> EpisodeSpan {
    EpisodeSpan::new(season, first, last).unwrap()
}

#[rstest]
#[case::with_year("Dune", Some(2021), "mkv", "Dune (2021)/Dune (2021).mkv")]
#[case::without_year("Dune", None, "mkv", "Dune/Dune.mkv")]
#[case::colon_becomes_dash("Dune: Part Two", Some(2024), "MKV", "Dune - Part Two (2024)/Dune - Part Two (2024).mkv")]
#[case::slash_is_not_a_folder("Face/Off", Some(1997), ".mp4", "Face-Off (1997)/Face-Off (1997).mp4")]
fn movie_paths_follow_jellyfin(
    #[case] title: &str,
    #[case] year: Option<i16>,
    #[case] extension: &str,
    #[case] expected: &str,
) {
    assert_eq!(Naming::default().movie_path(&movie(title, year), extension), PathBuf::from(expected));
}

#[rstest]
#[case::single(span(1, 2, 2), "Frieren (2023)/Season 01/Frieren (2023) - S01E02 - It Didn't Have to Be Magic.mkv")]
#[case::range(
    span(1, 1, 2),
    "Frieren (2023)/Season 01/Frieren (2023) - S01E01-E02 - The Journey's End + It Didn't Have to Be Magic.mkv"
)]
#[case::special(span(0, 1, 1), "Frieren (2023)/Season 00/Frieren (2023) - S00E01 - Recap.mkv")]
#[case::untitled(span(2, 1, 1), "Frieren (2023)/Season 02/Frieren (2023) - S02E01.mkv")]
fn episode_paths_follow_jellyfin(#[case] span: EpisodeSpan, #[case] expected: &str) {
    let frieren = series(
        "Frieren",
        Some(2023),
        &[(0, &["Recap"]), (1, &["The Journey's End", "It Didn't Have to Be Magic"]), (2, &[""])],
    );

    assert_eq!(Naming::default().episode_path(&frieren, span, "mkv"), Ok(PathBuf::from(expected)));
}

#[test]
fn episode_paths_reject_episodes_outside_the_series() {
    let frieren = series("Frieren", Some(2023), &[(1, &["One", "Two"])]);

    let result = Naming::default().episode_path(&frieren, span(1, 2, 3), "mkv");

    assert_eq!(result, Err(NamingError::UnknownEpisode(EpisodeRef { season: 1, episode: 3 })));
}

#[test]
fn custom_templates_shape_every_component() {
    let templates = NamingTemplates {
        series_folder: "{title}".into(),
        season_folder: "S{season}".into(),
        episode_file: "{episodes} {episode_title}".into(),
        ..NamingTemplates::default()
    };
    let naming = Naming::new(&templates).unwrap();
    let frieren = series("Frieren", Some(2023), &[(1, &["The Journey's End"])]);

    let path = naming.episode_path(&frieren, span(1, 1, 1), "mkv").unwrap();

    assert_eq!(path, PathBuf::from("Frieren/S01/S01E01 The Journey's End.mkv"));
}

#[test]
fn long_titles_keep_file_names_within_limits() {
    let title = "Very Long Title ".repeat(40);

    let path = Naming::default().movie_path(&movie(&title, Some(2021)), "mkv");

    let file_name = path.file_name().unwrap().to_str().unwrap();
    assert!(file_name.len() <= 205 && file_name.ends_with(".mkv"), "{file_name}");
    assert!(path.parent().unwrap().to_str().unwrap().len() <= 255);
}

fn components(path: &Path) -> Vec<String> {
    path.components().map(|component| component.as_os_str().to_string_lossy().into_owned()).collect()
}

proptest! {
    #[test]
    fn any_titles_give_clean_paths_of_fixed_depth(
        title in any::<String>(),
        episode_title in any::<String>(),
        year in prop::option::of(1900..2100i16),
        season in 0..30u16,
    ) {
        let naming = Naming::default();
        let series = series(&title, year, &[(season, &[episode_title.as_str()])]);

        let movie_path = naming.movie_path(&movie(&title, year), "mkv");
        let episode_path = naming.episode_path(&series, span(season, 1, 1), "mkv").unwrap();

        prop_assert_eq!(components(&movie_path).len(), 2);
        prop_assert_eq!(components(&episode_path).len(), 3);
        for component in components(&episode_path).iter().take(2).chain(components(&movie_path).first()) {
            prop_assert_eq!(&sanitize(component), component);
        }
    }
}
