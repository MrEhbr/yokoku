use std::path::PathBuf;

use jiff::{
    Timestamp,
    civil::{Date, date},
};
use rstest::rstest;
use yokoku_core::media::detect::{Conflict, ImportPlan, ListedFile, MatchScope};
use yokoku_domain::{
    Artwork, Confidence, Description, EpisodeMetadata, EpisodeSpan, ExternalId, FileTarget, ItemFolder, MediaFileId,
    MonitorPreset, Movie, MovieMetadata, Numbering, Releases, SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};

const TODAY: Date = date(2026, 9, 26);

struct SeriesSpec<'a> {
    source: u64,
    title: &'a str,
    original_title: &'a str,
    year: i16,
    /// `(season, episode titles)`
    seasons: &'a [(u16, &'a [&'a str])],
}

fn series(spec: SeriesSpec<'_>) -> Series {
    let mut source_id = spec.source * 1000;
    let metadata = SeriesMetadata {
        source: ExternalId::Tmdb(spec.source),
        title: spec.title.into(),
        original_title: spec.original_title.into(),
        alternate_titles: Vec::new(),
        year: Some(spec.year),
        artwork: Artwork::default(),
        description: Description::default(),
        status: SourceStatus::Returning,
        seasons: spec
            .seasons
            .iter()
            .map(|&(number, titles)| SeasonMetadata {
                number,
                episodes: titles
                    .iter()
                    .zip(1..)
                    .map(|(title, episode)| {
                        source_id += 1;
                        EpisodeMetadata {
                            source_id,
                            number: episode,
                            title: (*title).into(),
                            overview: String::new(),
                            air_date: None,
                        }
                    })
                    .collect(),
            })
            .collect(),
    };
    Series::new(metadata, ItemFolder::default(), MonitorPreset::All, TODAY, Timestamp::UNIX_EPOCH)
}

fn numbered(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("Episode {n}")).collect()
}

fn movie(source: u64, title: &str, original_title: &str, year: i16) -> Movie {
    let metadata = MovieMetadata {
        source: ExternalId::Tmdb(source),
        title: title.into(),
        original_title: original_title.into(),
        alternate_titles: Vec::new(),
        year: Some(year),
        artwork: Artwork::default(),
        description: Description::default(),
        releases: Releases::default(),
    };
    Movie::new(metadata, ItemFolder::default(), true, Timestamp::UNIX_EPOCH)
}

struct Library {
    series: Vec<Series>,
    movies: Vec<Movie>,
}

impl Library {
    fn new() -> Self {
        let s1 = numbered(28);
        let s2 = numbered(10);
        let s1: Vec<&str> = s1.iter().map(String::as_str).collect();
        let s2: Vec<&str> = s2.iter().map(String::as_str).collect();
        let mut frieren = series(SeriesSpec {
            source: 1,
            title: "Frieren: Beyond Journey's End",
            original_title: "葬送のフリーレン",
            year: 2023,
            seasons: &[(0, &["Recap"]), (1, &s1), (2, &s2)],
        });
        frieren.alternate_titles = vec!["Sousou no Frieren".into(), "Sōsō no Furīren".into()];
        frieren.numbering = Numbering::Absolute;

        let breaking_bad = series(SeriesSpec {
            source: 2,
            title: "Breaking Bad",
            original_title: "Breaking Bad",
            year: 2008,
            seasons: &[
                (1, &["Pilot", "Cat's in the Bag...", "...And the Bag's in the River"]),
                (2, &["Seven Thirty-Seven", "Grilled", "Bit by a Dead Bee", "Down", "Breakage", "Peekaboo"]),
            ],
        });
        let mut daily_show = series(SeriesSpec {
            source: 3,
            title: "The Daily Show",
            original_title: "The Daily Show",
            year: 1996,
            seasons: &[(31, &["Guest A", "Guest B", "Guest C"])],
        });
        for (episode, day) in daily_show.seasons[0].episodes.iter_mut().zip([24, 25, 26]) {
            episode.air_date = Some(date(2026, 9, day));
        }
        let old_who = series(SeriesSpec {
            source: 4,
            title: "Doctor Who",
            original_title: "Doctor Who",
            year: 1963,
            seasons: &[(1, &["An Unearthly Child"])],
        });
        let new_who = series(SeriesSpec {
            source: 5,
            title: "Doctor Who",
            original_title: "Doctor Who",
            year: 2005,
            seasons: &[(1, &["Rose"])],
        });
        let the_method = series(SeriesSpec {
            source: 6,
            title: "The Method",
            original_title: "Метод",
            year: 2015,
            seasons: &[(1, &["Пилот", "Вторая"])],
        });

        Self {
            series: vec![frieren, breaking_bad, daily_show, old_who, new_who, the_method],
            movies: vec![
                movie(10, "Dune", "Dune", 2021),
                movie(11, "Dune: Part Two", "Dune: Part Two", 2024),
                movie(12, "Blade Runner 2049", "Blade Runner 2049", 2017),
                Movie {
                    alternate_titles: vec!["Amelie from Montmartre".into()],
                    ..movie(13, "Amélie", "Le Fabuleux Destin d'Amélie Poulain", 2001)
                },
            ],
        }
    }

    fn series(&self, source: u64) -> &Series {
        self.series.iter().find(|series| series.source == ExternalId::Tmdb(source)).unwrap()
    }

    fn movie(&self, source: u64) -> &Movie {
        self.movies.iter().find(|movie| movie.source == ExternalId::Tmdb(source)).unwrap()
    }

    fn unlinked(&self) -> MatchScope<'_> {
        MatchScope::Library { series: &self.series, movies: &self.movies }
    }
}

fn files(paths: &[&str]) -> Vec<ListedFile> {
    paths.iter().map(|path| ListedFile { path: PathBuf::from(path), size: 1_000 }).collect()
}

fn episodes(series: &Series, season: u16, first: u16, last: u16) -> Option<FileTarget> {
    Some(FileTarget::Episodes { series: series.id, span: EpisodeSpan::new(season, first, last).unwrap() })
}

fn outcome(plan: &ImportPlan) -> Vec<(Option<FileTarget>, Confidence)> {
    plan.rows.iter().map(|row| (row.target, row.confidence)).collect()
}

#[test]
fn linked_series_matches_by_numbers_and_imports_automatically() {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(&files(&["Breaking.Bad.S02E03.720p.mkv"]), MatchScope::Series(bad));

    assert_eq!(outcome(&plan), [(episodes(bad, 2, 3, 3), Confidence::Certain)]);
    assert!(plan.is_automatic());
}

#[test]
fn linked_series_ignores_the_title_in_the_name() {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(&files(&["Totally.Different.Name.S01E02.mkv"]), MatchScope::Series(bad));

    assert_eq!(outcome(&plan), [(episodes(bad, 1, 2, 2), Confidence::Certain)]);
}

#[rstest]
#[case::episode_does_not_exist("Breaking.Bad.S02E99.mkv")]
#[case::season_does_not_exist("Breaking.Bad.S09E01.mkv")]
#[case::gap_in_episodes("Breaking.Bad.S01E01E03.mkv")]
#[case::nothing_to_go_on("readme video.mkv")]
fn names_that_do_not_fit_the_episode_list_are_unknown(#[case] name: &str) {
    let library = Library::new();

    let plan = ImportPlan::new(&files(&[name]), MatchScope::Series(library.series(2)));

    assert_eq!(outcome(&plan), [(None, Confidence::Unknown)]);
    assert!(!plan.is_automatic());
}

#[test]
fn season_packs_take_the_season_from_the_folder() {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(
        &files(&["Breaking.Bad.S02.1080p/Breaking.Bad.E05.mkv", "Breaking.Bad.S02.1080p/06.mkv"]),
        MatchScope::Series(bad),
    );

    assert_eq!(
        outcome(&plan),
        [(episodes(bad, 2, 6, 6), Confidence::Certain), (episodes(bad, 2, 5, 5), Confidence::Certain)]
    );
}

#[test]
fn full_series_packs_use_each_season_folder() {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(
        &files(&["Breaking Bad/Season 1/01.mkv", "Breaking Bad/Season 2/01.mkv"]),
        MatchScope::Series(bad),
    );

    assert_eq!(
        outcome(&plan),
        [(episodes(bad, 1, 1, 1), Confidence::Certain), (episodes(bad, 2, 1, 1), Confidence::Certain)]
    );
}

#[test]
fn multi_episode_files_match_a_span() {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(&files(&["Breaking.Bad.S01E01-E03.mkv"]), MatchScope::Series(bad));

    assert_eq!(outcome(&plan), [(episodes(bad, 1, 1, 3), Confidence::Certain)]);
}

#[rstest]
#[case::first_season("[SubsPlease] Sousou no Frieren - 12 (1080p) [ABCD1234].mkv", 1, 12)]
#[case::second_season("[SubsPlease] Sousou no Frieren - 30 (1080p) [ABCD1234].mkv", 2, 2)]
fn absolute_numbers_convert_for_absolute_series(#[case] name: &str, #[case] season: u16, #[case] episode: u16) {
    let library = Library::new();
    let frieren = library.series(1);

    let plan = ImportPlan::new(&files(&[name]), library.unlinked());

    assert_eq!(outcome(&plan), [(episodes(frieren, season, episode, episode), Confidence::Certain)]);
}

#[test]
fn seasonless_numbers_for_a_standard_series_are_a_guess() {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(&files(&["Breaking Bad - 04.mkv"]), MatchScope::Series(bad));

    assert_eq!(outcome(&plan), [(episodes(bad, 2, 1, 1), Confidence::Guess)]);
}

#[rstest]
#[case::seasonless_name("Breaking Bad - 01.mkv", 2, 1)]
#[case::name_with_a_season("Breaking.Bad.S01E02.mkv", 1, 2)]
fn a_season_given_with_the_series_places_names_without_one(
    #[case] name: &str,
    #[case] season: u16,
    #[case] episode: u16,
) {
    let library = Library::new();
    let bad = library.series(2);

    let plan = ImportPlan::new(&files(&[name]), MatchScope::SeriesSeason { series: bad, season: 2 });

    assert_eq!(outcome(&plan), [(episodes(bad, season, episode, episode), Confidence::Certain)]);
}

#[test]
fn daily_shows_match_by_air_date() {
    let library = Library::new();
    let daily = library.series(3);

    let plan =
        ImportPlan::new(&files(&["The.Daily.Show.2026.09.25.Guest.B.720p.WEB.h264-EDITH.mkv"]), library.unlinked());

    assert_eq!(outcome(&plan), [(episodes(daily, 31, 2, 2), Confidence::Certain)]);
}

#[rstest]
#[case::english("Breaking Bad/Season 2/Bit by a Dead Bee.mkv", 2, 2, 3)]
#[case::russian("Метод/Пилот.mkv", 6, 1, 1)]
fn names_without_numbers_match_by_episode_title(
    #[case] name: &str,
    #[case] source: u64,
    #[case] season: u16,
    #[case] episode: u16,
) {
    let library = Library::new();
    let series = library.series(source);

    let plan = ImportPlan::new(&files(&[name]), MatchScope::Series(series));

    assert_eq!(outcome(&plan), [(episodes(series, season, episode, episode), Confidence::Certain)]);
}

/// `expected` is `(series source, season, episode)`.
#[rstest]
#[case::exact_title("Breaking.Bad.S01E01.720p.mkv", Some((2, 1, 1)), Confidence::Certain)]
#[case::original_title("Метод.S01E02.WEB-DL.mkv", Some((6, 1, 2)), Confidence::Certain)]
#[case::alternate_title("Sousou.no.Frieren.S01E03.1080p.mkv", Some((1, 1, 3)), Confidence::Certain)]
#[case::romanised_title("Soso.no.Furiren.S01E03.1080p.mkv", Some((1, 1, 3)), Confidence::Certain)]
#[case::year_picks_the_series("Doctor.Who.2005.S01E01.720p.mkv", Some((5, 1, 1)), Confidence::Certain)]
#[case::close_title("Breaking.Bd.S01E01.720p.mkv", Some((2, 1, 1)), Confidence::Guess)]
#[case::ambiguous_title("Doctor.Who.S01E01.720p.mkv", None, Confidence::Unknown)]
#[case::not_in_library("Severance.S01E01.720p.mkv", None, Confidence::Unknown)]
fn unlinked_series_match_by_title(
    #[case] name: &str,
    #[case] expected: Option<(u64, u16, u16)>,
    #[case] confidence: Confidence,
) {
    let library = Library::new();

    let plan = ImportPlan::new(&files(&[name]), library.unlinked());

    let target =
        expected.and_then(|(source, season, episode)| episodes(library.series(source), season, episode, episode));
    assert_eq!(outcome(&plan), [(target, confidence)]);
}

#[rstest]
#[case::exact("Dune.2021.1080p.BluRay.x264.mkv", 10, Confidence::Certain)]
#[case::year_rules_out_the_first_film("Dune.Part.Two.2024.1080p.WEB-DL.mkv", 11, Confidence::Guess)]
#[case::number_in_the_title("Blade.Runner.2049.2017.2160p.UHD.BluRay.x265.mkv", 12, Confidence::Guess)]
#[case::accents_are_ignored("Amelie.2001.1080p.BluRay.mkv", 13, Confidence::Certain)]
#[case::original_title("Le.Fabuleux.Destin.d.Amelie.Poulain.2001.mkv", 13, Confidence::Certain)]
#[case::alternate_title("Amelie.from.Montmartre.2001.1080p.mkv", 13, Confidence::Certain)]
#[case::release_year_one_off("Amelie.2002.1080p.mkv", 13, Confidence::Guess)]
fn unlinked_movies_match_by_title_and_year(#[case] name: &str, #[case] source: u64, #[case] confidence: Confidence) {
    let library = Library::new();

    let plan = ImportPlan::new(&files(&[name]), library.unlinked());

    assert_eq!(outcome(&plan), [(Some(FileTarget::Movie(library.movie(source).id)), confidence)]);
}

#[test]
fn movies_keep_the_largest_video_and_ignore_the_rest() {
    let library = Library::new();
    let dune = library.movie(10);
    let files = vec![
        ListedFile { path: "Dune.2021/Dune.2021.1080p.mkv".into(), size: 8_000_000_000 },
        ListedFile { path: "Dune.2021/Dune.2021.1080p.en.srt".into(), size: 90_000 },
        ListedFile { path: "Dune.2021/Behind.The.Dune.mkv".into(), size: 300_000_000 },
        ListedFile { path: "Dune.2021/Dune.2021.sample.mkv".into(), size: 50_000_000 },
    ];

    let plan = ImportPlan::new(&files, MatchScope::Movie(dune));

    assert_eq!(outcome(&plan), [(Some(FileTarget::Movie(dune.id)), Confidence::Certain)]);
    assert_eq!(plan.rows[0].video.subtitles.len(), 1);
    let mut ignored: Vec<_> = plan.ignored.iter().map(|path| path.to_str().unwrap()).collect();
    ignored.sort_unstable();
    assert_eq!(ignored, ["Dune.2021/Behind.The.Dune.mkv", "Dune.2021/Dune.2021.sample.mkv"]);
}

#[test]
fn episodes_that_already_have_files_are_conflicts() {
    let mut library = Library::new();
    library.series[1].seasons[0].episodes[0].file = Some(MediaFileId::generate());
    let bad = library.series(2);

    let plan = ImportPlan::new(&files(&["Breaking.Bad.S01E01.mkv"]), MatchScope::Series(bad));

    assert_eq!(plan.rows[0].conflicts, [Conflict::AlreadyHasFile]);
    assert!(!plan.is_automatic());
}

#[test]
fn two_files_for_the_same_episode_are_conflicts() {
    let library = Library::new();

    let plan = ImportPlan::new(
        &files(&["Breaking.Bad.S01E01-E02.mkv", "Breaking.Bad.S01E02.1080p.mkv", "Breaking.Bad.S01E03.mkv"]),
        MatchScope::Series(library.series(2)),
    );

    let conflicts: Vec<_> = plan.rows.iter().map(|row| row.conflicts.clone()).collect();
    assert_eq!(conflicts, [vec![Conflict::SharedTarget], vec![Conflict::SharedTarget], vec![]]);
    assert!(!plan.is_automatic());
}

#[test]
fn an_empty_download_is_never_automatic() {
    let library = Library::new();

    let plan = ImportPlan::new(&files(&["readme.txt"]), MatchScope::Series(library.series(2)));

    assert!(plan.rows.is_empty());
    assert!(!plan.is_automatic());
}

#[test]
fn air_dates_outside_the_list_do_not_match() {
    let library = Library::new();

    let plan = ImportPlan::new(&files(&["The.Daily.Show.2026.09.27.720p.mkv"]), MatchScope::Series(library.series(3)));

    assert_eq!(outcome(&plan), [(None, Confidence::Unknown)]);
}
