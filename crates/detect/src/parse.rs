use std::{borrow::Cow, path::Path, sync::LazyLock};

use hunch::{HunchResult, Property, hunch};
use jiff::civil::Date;
use regex::Regex;

/// A file stem that starts with a bare episode number: `05`, `03 - Pilot`, `03.Grilled`.
static BARE_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]{1,3}(?:[\s._-]+[^\s._\-0-9(].*)?$").expect("valid regex"));

/// Jellyfin naming: `Title (Year)`, optionally followed by ` - ` and the rest of the name.
static TITLE_WITH_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<title>.+?) \((?P<year>[0-9]{4})\)(?: - .+)?$").expect("valid regex"));

/// What a file's name and folders say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedName {
    pub title: Option<String>,
    pub year: Option<i16>,
    pub numbers: Numbers,
    pub episode_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Numbers {
    None,
    /// Episodes are sorted and non-empty.
    Episodes {
        season: u16,
        episodes: Vec<u16>,
    },
    /// Episode numbers without a season in the name or folders: absolute numbers, or episodes of an unknown season.
    Seasonless {
        episodes: Vec<u16>,
    },
    Date(Date),
}

impl ParsedName {
    /// Parses a file path relative to its download or root folder.
    pub fn parse(path: &Path) -> Self {
        let result = hunch(&Self::hunch_input(path));
        let date = result.date().and_then(|date| date.parse::<Date>().ok());
        let (title, year) = match Self::title_with_year(path) {
            Some((title, year)) => (Some(title), year),
            None if date.is_some() => (result.title().map(str::to_owned), None),
            None => (result.title().map(str::to_owned), result.year().and_then(|year| year.try_into().ok())),
        };

        Self {
            title,
            year,
            numbers: date.map_or_else(|| Numbers::from(&result), Numbers::Date),
            episode_title: result.episode_title().map(str::to_owned),
        }
    }

    /// `Title (Year)` from the file stem, or from the folders above a bare-number file.
    fn title_with_year(path: &Path) -> Option<(String, Option<i16>)> {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let folders = path.parent().into_iter().flat_map(Path::iter).rev().map(|folder| folder.to_string_lossy());
        let folders = folders.filter(|_| BARE_NUMBER.is_match(&stem));
        std::iter::once(stem.clone()).chain(folders).find_map(|name| {
            let named = TITLE_WITH_YEAR.captures(&name)?;
            Some((named["title"].to_owned(), named["year"].parse().ok()))
        })
    }

    /// The path with `Specials` folders as season 0 and a bare leading episode number marked `E`.
    fn hunch_input(path: &Path) -> String {
        let folders = path.parent().into_iter().flat_map(Path::iter).map(|folder| match folder.to_string_lossy() {
            folder if ["special", "specials"].iter().any(|name| folder.eq_ignore_ascii_case(name)) => {
                Cow::Borrowed("Season 0")
            },
            folder => folder,
        });
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let file_name = if BARE_NUMBER.is_match(&stem) { Cow::Owned(format!("E{file_name}")) } else { file_name };
        folders.chain([file_name]).collect::<Vec<_>>().join("/")
    }
}

impl From<&HunchResult> for Numbers {
    fn from(result: &HunchResult) -> Self {
        let numbers = |property| -> Vec<u16> { result.all(property).iter().filter_map(|n| n.parse().ok()).collect() };
        let mut episodes = numbers(Property::Episode);
        if episodes.is_empty() {
            episodes = numbers(Property::AbsoluteEpisode);
        }
        episodes.sort_unstable();
        episodes.dedup();

        match result.season().and_then(|season| u16::try_from(season).ok()) {
            _ if episodes.is_empty() => Self::None,
            Some(season) => Self::Episodes { season, episodes },
            None => Self::Seasonless { episodes },
        }
    }
}
