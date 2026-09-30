use jiff::civil::Date;
use serde_json::Value;
use yokoku_domain::{
    Artwork, Description, EpisodeMetadata, ExternalId, MovieMetadata, Releases, SeasonMetadata, SeriesMetadata,
    SourceStatus,
};

/// A recorded TMDB or TVDB answer from `crates/metadata/tests/fixtures`.
pub fn fixture(name: &str) -> Value {
    let file = format!("{}/../metadata/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// A 2023 series at TMDB, with seasons as `(number, air dates)`; episode source ids are
/// `source * 1000` plus a count across seasons.
pub fn series_metadata(
    source: u64,
    title: &str,
    status: SourceStatus,
    seasons: &[(u16, &[Option<Date>])],
) -> SeriesMetadata {
    let mut next_source_id = source * 1000;
    SeriesMetadata {
        source: ExternalId::Tmdb(source),
        title: title.into(),
        original_title: title.into(),
        alternate_titles: Vec::new(),
        year: Some(2023),
        artwork: Artwork::default(),
        description: Description::default(),
        status,
        seasons: seasons
            .iter()
            .map(|&(number, dates)| SeasonMetadata {
                number,
                episodes: dates
                    .iter()
                    .zip(1..)
                    .map(|(&air_date, episode)| {
                        next_source_id += 1;
                        EpisodeMetadata {
                            source_id: next_source_id,
                            number: episode,
                            title: format!("Episode {episode}"),
                            overview: String::new(),
                            air_date,
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// A movie at TMDB without a year.
pub fn movie_metadata(source: u64, title: &str, releases: Releases) -> MovieMetadata {
    MovieMetadata {
        source: ExternalId::Tmdb(source),
        title: title.into(),
        original_title: title.into(),
        alternate_titles: Vec::new(),
        year: None,
        artwork: Artwork::default(),
        description: Description::default(),
        releases,
    }
}
