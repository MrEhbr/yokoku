//! TheTVDB v4 response shapes; only the fields Yokoku reads.

use std::collections::BTreeMap;

use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ImdbId, SourceStatus};

#[derive(Debug, Serialize)]
pub(crate) struct Login<'a> {
    pub apikey: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Token {
    pub token: String,
}

/// Every answer wraps its record in `data`; paged answers link the next page.
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    pub data: T,
    #[serde(default)]
    pub links: Links,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Links {
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchItem {
    pub tvdb_id: String,
    pub name: String,
    #[serde(default, deserialize_with = "crate::metadata::dates::year")]
    pub year: Option<i16>,
    pub image_url: Option<String>,
    /// Names by three-letter language code.
    #[serde(default)]
    pub translations: BTreeMap<String, String>,
    /// In the original language.
    pub overview: Option<String>,
    /// Overviews by three-letter language code.
    #[serde(default)]
    pub overviews: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SeriesDetails {
    pub name: String,
    /// In the original language.
    pub overview: Option<String>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Minutes.
    pub average_runtime: Option<u16>,
    #[serde(default, deserialize_with = "crate::metadata::dates::year")]
    pub year: Option<i16>,
    pub image: Option<String>,
    pub status: Option<Status>,
    #[serde(default)]
    pub aliases: Vec<Alias>,
    #[serde(default)]
    pub remote_ids: Vec<RemoteId>,
    #[serde(default)]
    pub translations: Translations,
}

impl SeriesDetails {
    pub(crate) fn imdb_id(&self) -> Option<ImdbId> {
        self.remote_ids.iter().find(|remote| remote.source_name == "IMDB")?.id.parse().ok()
    }
}

/// The series' id at another site.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteId {
    pub id: String,
    pub source_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Status {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Alias {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Genre {
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Translations {
    #[serde(default)]
    pub name_translations: Vec<Translation>,
    #[serde(default)]
    pub overview_translations: Vec<OverviewTranslation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Translation {
    pub language: String,
    pub name: Option<String>,
    /// `true` for other names in the language; the translated title has `null`.
    pub is_alias: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OverviewTranslation {
    pub language: String,
    pub overview: Option<String>,
}

impl SeriesDetails {
    pub(crate) fn translated_name(&self, language: &str) -> Option<&str> {
        self.translations
            .name_translations
            .iter()
            .find(|translation| translation.language == language && translation.is_alias != Some(true))
            .and_then(|translation| translation.name.as_deref())
            .filter(|name| !name.is_empty())
    }

    /// The overview in `language`, else in the original language.
    pub(crate) fn overview(&self, language: &str) -> Option<&str> {
        self.translations
            .overview_translations
            .iter()
            .find(|translation| translation.language == language)
            .and_then(|translation| translation.overview.as_deref())
            .or(self.overview.as_deref())
            .filter(|overview| !overview.is_empty())
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct EpisodePage {
    pub episodes: Vec<EpisodeItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpisodeItem {
    pub id: u64,
    pub season_number: u16,
    pub number: u16,
    pub name: Option<String>,
    /// In the requested language.
    pub overview: Option<String>,
    #[serde(default, deserialize_with = "crate::metadata::dates::date")]
    pub aired: Option<Date>,
}

pub(crate) fn source_status(value: Option<&str>) -> SourceStatus {
    match value {
        Some("Continuing") => SourceStatus::Returning,
        Some("Upcoming") => SourceStatus::Planned,
        Some("Ended") => SourceStatus::Ended,
        _ => SourceStatus::Unknown,
    }
}

/// Series artwork type ids from `artwork/types`.
pub(crate) const SERIES_BACKGROUND: u32 = 3;
pub(crate) const SERIES_CLEAR_LOGO: u32 = 23;

/// `series/{id}/artworks`: every artwork of the series.
#[derive(Debug, Deserialize)]
pub(crate) struct SeriesArtworks {
    #[serde(default)]
    pub artworks: Vec<ArtworkItem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArtworkItem {
    pub image: String,
    #[serde(rename = "type")]
    pub kind: u32,
    /// Three-letter language code; `None` for artwork without text.
    pub language: Option<String>,
    #[serde(default)]
    pub score: f64,
}

impl SeriesArtworks {
    /// The best-scored artwork of `kind` in the first of `languages` that has one; `None` stands
    /// for artwork without text.
    pub(crate) fn best(&self, kind: u32, languages: [Option<&str>; 2]) -> Option<String> {
        languages.into_iter().find_map(|language| {
            self.artworks
                .iter()
                .filter(|artwork| artwork.kind == kind && artwork.language.as_deref() == language)
                .max_by(|a, b| a.score.total_cmp(&b.score))
                .map(|artwork| artwork.image.clone())
        })
    }
}
