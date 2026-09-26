use jiff::{Timestamp, civil::date};
use proptest::prelude::*;
use yokoku_detect::{DownloadFile, Target, plan};
use yokoku_domain::{
    Confidence, EpisodeMetadata, EpisodeSpan, ExternalId, FileTarget, MonitorPreset, Movie, MovieMetadata, Releases,
    SeasonMetadata, Series, SeriesMetadata, SourceStatus,
};
use yokoku_naming::Naming;

const TITLE: &str = "[A-Za-z][A-Za-z0-9 :'&.,!?-]{0,30}";
const EPISODE_TITLE: &str = "[A-Za-z ]{0,20}";

fn series(title: String, year: i16, episode_titles: Vec<String>) -> Series {
    let episodes = (1..)
        .zip(episode_titles)
        .map(|(number, title)| EpisodeMetadata { source_id: u64::from(number), number, title, air_date: None })
        .collect();
    let metadata = SeriesMetadata {
        source: ExternalId::Tmdb(1),
        title: title.clone(),
        original_title: title,
        alternate_titles: Vec::new(),
        year: Some(year),
        poster_path: None,
        status: SourceStatus::Returning,
        seasons: vec![SeasonMetadata { number: 1, episodes }],
    };
    Series::add(metadata, MonitorPreset::All, date(2026, 9, 26), Timestamp::UNIX_EPOCH)
}

fn movie(title: String, year: i16) -> Movie {
    let metadata = MovieMetadata {
        source: ExternalId::Tmdb(2),
        title: title.clone(),
        original_title: title,
        alternate_titles: Vec::new(),
        year: Some(year),
        poster_path: None,
        releases: Releases::default(),
    };
    Movie::add(metadata, true, Timestamp::UNIX_EPOCH)
}

proptest! {
    #[test]
    fn named_episodes_are_detected_with_certainty(
        title in TITLE,
        year in 1950..2030i16,
        episode_titles in prop::collection::vec(EPISODE_TITLE, 3),
        first in 1..=3u16,
        length in 0..3u16,
    ) {
        let series = series(title, year, episode_titles);
        let span = EpisodeSpan::new(1, first, (first + length).min(3)).unwrap();
        let path = Naming::default().episode_path(&series, span, "mkv").unwrap();

        let plan = plan(&[DownloadFile { path: path.clone(), size: 1 }], Target::Library {
            series: std::slice::from_ref(&series),
            movies: &[],
        });

        prop_assert_eq!(plan.rows.len(), 1, "{}", path.display());
        prop_assert_eq!(plan.rows[0].target, Some(FileTarget::Episodes { series: series.id, span }), "{}", path.display());
        prop_assert_eq!(plan.rows[0].confidence, Confidence::Certain, "{}", path.display());
    }

    #[test]
    fn named_movies_are_detected_with_certainty(title in TITLE, year in 1950..2030i16) {
        let movie = movie(title, year);
        let path = Naming::default().movie_path(&movie, "mkv");

        let plan = plan(&[DownloadFile { path: path.clone(), size: 1 }], Target::Library {
            series: &[],
            movies: std::slice::from_ref(&movie),
        });

        prop_assert_eq!(plan.rows.len(), 1, "{}", path.display());
        prop_assert_eq!(plan.rows[0].target, Some(FileTarget::Movie(movie.id)), "{}", path.display());
        prop_assert_eq!(plan.rows[0].confidence, Confidence::Certain, "{}", path.display());
    }
}
