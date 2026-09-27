//! TheTVDB v4 response shapes; only the fields Yokoku reads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yokoku_domain::SourceStatus;

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
    pub year: Option<String>,
    pub image_url: Option<String>,
    /// Names by three-letter language code.
    #[serde(default)]
    pub translations: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SeriesDetails {
    pub name: String,
    pub year: Option<String>,
    pub image: Option<String>,
    pub status: Option<Status>,
    #[serde(default)]
    pub aliases: Vec<Alias>,
    #[serde(default)]
    pub translations: Translations,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Status {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Alias {
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Translations {
    #[serde(default)]
    pub name_translations: Vec<Translation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Translation {
    pub language: String,
    pub name: Option<String>,
    /// `true` for other names in the language; the translated title has `null`.
    pub is_alias: Option<bool>,
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
    pub aired: Option<String>,
}

pub(crate) fn source_status(value: Option<&str>) -> SourceStatus {
    match value {
        Some("Continuing") => SourceStatus::Returning,
        Some("Upcoming") => SourceStatus::Planned,
        Some("Ended") => SourceStatus::Ended,
        _ => SourceStatus::Unknown,
    }
}
