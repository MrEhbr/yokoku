//! TMDB response shapes; only the fields Yokoku reads.

use std::collections::HashMap;

use jiff::civil::Date;
use serde::Deserialize;
use yokoku_domain::SourceStatus;

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
    pub status: Option<String>,
    pub seasons: Vec<SeasonSummary>,
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
    pub release_dates: Option<ReleaseDatesByCountry>,
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

/// `2021-10-22` or `2021-10-22T00:00:00.000Z`; empty strings are missing dates.
pub(crate) fn date(value: Option<&str>) -> Option<Date> {
    value?.get(..10)?.parse().ok()
}

pub(crate) fn year(value: Option<&str>) -> Option<i16> {
    value?.get(..4)?.parse().ok()
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
