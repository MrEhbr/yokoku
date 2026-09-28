//! TMDB response shapes; only the fields Yokoku reads.

use std::collections::HashMap;

use serde::Deserialize;
use yokoku_domain::{Releases, SourceStatus};

use crate::http::date;

#[derive(Debug, Deserialize)]
pub(crate) struct SearchPage {
    pub results: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "media_type", rename_all = "lowercase")]
pub(crate) enum SearchItem {
    Movie(MovieSummary),
    Tv(TvSummary),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MovieSummary {
    pub id: u64,
    pub title: String,
    pub original_title: String,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TvSummary {
    pub id: u64,
    pub name: String,
    pub original_name: String,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TvDetails {
    pub name: String,
    pub original_name: String,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    #[serde(default)]
    pub images: Images,
    pub status: Option<String>,
    pub seasons: Vec<SeasonSummary>,
    #[serde(default)]
    pub alternative_titles: AlternativeTitles,
    /// Appended `season/N` objects, among other fields.
    #[serde(flatten)]
    pub appended: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SeasonSummary {
    pub season_number: u16,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SeasonDetails {
    pub season_number: u16,
    pub episodes: Vec<EpisodeItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EpisodeItem {
    pub id: u64,
    pub episode_number: u16,
    pub name: String,
    pub air_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MovieDetails {
    pub title: String,
    pub original_title: String,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    #[serde(default)]
    pub images: Images,
    pub release_dates: Option<ReleaseDatesByCountry>,
    #[serde(default)]
    pub alternative_titles: AlternativeTitles,
}

impl MovieDetails {
    /// Earliest date per kind in `region`; the primary release date stands in for a missing cinema date.
    pub(crate) fn releases(&self, region: &str) -> Releases {
        let dates: Vec<_> = self
            .release_dates
            .iter()
            .flat_map(|by_country| &by_country.results)
            .filter(|country| country.iso_3166_1.eq_ignore_ascii_case(region))
            .flat_map(|country| &country.release_dates)
            .filter_map(|release| Some((release.kind, date(Some(&release.release_date))?)))
            .collect();
        let earliest =
            |kinds: &[u8]| dates.iter().filter(|(kind, _)| kinds.contains(kind)).map(|&(_, date)| date).min();

        Releases {
            cinema: earliest(&[2, 3]).or_else(|| date(self.release_date.as_deref())),
            digital: earliest(&[4]),
            physical: earliest(&[5]),
        }
    }
}

/// `results` for series, `titles` for movies.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct AlternativeTitles {
    #[serde(alias = "titles")]
    pub results: Vec<AlternativeTitle>,
}

impl AlternativeTitles {
    /// Distinct titles other than `title` and `original_title`, in TMDB's order.
    pub(crate) fn into_distinct(self, title: &str, original_title: &str) -> Vec<String> {
        let mut titles: Vec<String> = Vec::new();
        for alternative in self.results {
            if alternative.title != title && alternative.title != original_title && !titles.contains(&alternative.title)
            {
                titles.push(alternative.title);
            }
        }
        titles
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AlternativeTitle {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReleaseDatesByCountry {
    pub results: Vec<CountryReleases>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CountryReleases {
    pub iso_3166_1: String,
    pub release_dates: Vec<ReleaseDate>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReleaseDate {
    pub release_date: String,
    /// 1 premiere, 2 limited theatrical, 3 theatrical, 4 digital, 5 physical, 6 TV.
    #[serde(rename = "type")]
    pub kind: u8,
}

pub(crate) fn source_status(value: Option<&str>) -> SourceStatus {
    match value {
        Some("Returning Series") => SourceStatus::Returning,
        Some("Planned") => SourceStatus::Planned,
        Some("In Production") => SourceStatus::InProduction,
        Some("Pilot") => SourceStatus::Pilot,
        Some("Ended") => SourceStatus::Ended,
        Some("Canceled" | "Cancelled") => SourceStatus::Canceled,
        _ => SourceStatus::Unknown,
    }
}

/// Appended `images`, in the languages the request names.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Images {
    #[serde(default)]
    pub logos: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Image {
    pub file_path: String,
    /// Two-letter language code; `None` for images without text.
    pub iso_639_1: Option<String>,
    #[serde(default)]
    pub vote_average: f64,
}

impl Images {
    /// The best-voted logo in `language` (two letters), else the best-voted one without a language.
    pub(crate) fn logo(&self, language: &str) -> Option<String> {
        let best = |in_language: &dyn Fn(Option<&str>) -> bool| {
            self.logos
                .iter()
                .filter(|logo| in_language(logo.iso_639_1.as_deref()))
                .max_by(|a, b| a.vote_average.total_cmp(&b.vote_average))
                .map(|logo| logo.file_path.clone())
        };
        best(&|code| code == Some(language)).or_else(|| best(&|code| code.is_none()))
    }
}
