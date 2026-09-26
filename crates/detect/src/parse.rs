use std::{path::Path, sync::LazyLock};

use hunch::{HunchResult, Property, hunch};
use jiff::civil::Date;
use regex::Regex;

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
    /// Episode numbers without a season in the file name: absolute numbers, or episodes of the folder's season.
    Seasonless {
        episodes: Vec<u16>,
        folder_season: Option<u16>,
    },
    Date(Date),
}

/// Russian `1 сезон 3 серия` or `Сезон 1 Серия 3`.
static RUSSIAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?P<title>.*?)[\s._-]*(?:(?P<s1>[0-9]{1,3})[\s._-]*сезон|сезон[\s._-]*(?P<s2>[0-9]{1,3}))[\s.,_-]*(?:(?P<e1>[0-9]{1,4})[\s._-]*серия|серия[\s._-]*(?P<e2>[0-9]{1,4}))",
    )
    .expect("valid regex")
});

/// A stem that is only an episode number, optionally followed by a title: `05`, `03 - Pilot`.
static BARE_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<episode>[0-9]{1,3})(?:[\s._-]+(?P<title>.+))?$").expect("valid regex"));

static FOLDER_SEASON: LazyLock<[Regex; 4]> = LazyLock::new(|| {
    [
        r"(?i)(?:^|[^\p{L}\p{N}])s(?P<season>[0-9]{1,2})(?:[^\p{L}\p{N}]|$)",
        r"(?i)season[\s._-]*(?P<season>[0-9]{1,3})",
        r"(?i)сезон[\s._-]*(?P<season>[0-9]{1,3})",
        r"(?i)(?P<season>[0-9]{1,3})[\s._-]*сезон",
    ]
    .map(|pattern| Regex::new(pattern).expect("valid regex"))
});

static SPECIALS_FOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:specials?|спецвыпуски)$").expect("valid regex"));

/// A folder named only by its season: `S02`, `Season 2`, `Сезон 2`, `2 сезон`, `Specials`.
static SEASON_ONLY_FOLDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:s[0-9]{1,2}|(?:season|сезон)[\s._-]*[0-9]{1,3}|[0-9]{1,3}[\s._-]*сезон|specials?|спецвыпуски)$",
    )
    .expect("valid regex")
});

/// Jellyfin naming: `Title (Year)`, optionally followed by ` - ` and the rest of the name.
static TITLE_WITH_YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<title>.+?) \((?P<year>[0-9]{4})\)(?: - (?P<rest>.+))?$").expect("valid regex"));

/// Parses a file path relative to its download or root folder.
pub fn parse(path: &Path) -> ParsedName {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path.extension().map(|extension| extension.to_string_lossy());
    let folders: Vec<String> = path
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .rev()
        .collect();
    let folder_season = folders.iter().find_map(|folder| folder_season(folder));
    let folder_title = || {
        folders.iter().filter(|folder| !SEASON_ONLY_FOLDER.is_match(folder.trim())).find_map(|folder| {
            match TITLE_WITH_YEAR.captures(folder) {
                Some(named) if named.name("rest").is_none() => Some(named["title"].to_owned()),
                _ => hunch(folder).title().map(str::to_owned),
            }
        })
    };

    if let Some(named) = TITLE_WITH_YEAR.captures(&stem) {
        let parsed = match named.name("rest") {
            Some(rest) => parse_name(rest.as_str(), extension.as_deref(), folder_season, || None),
            None => ParsedName { title: None, year: None, numbers: Numbers::None, episode_title: None },
        };
        return ParsedName { title: Some(named["title"].to_owned()), year: named["year"].parse().ok(), ..parsed };
    }
    parse_name(&stem, extension.as_deref(), folder_season, folder_title)
}

/// Parses a file stem; `folder_title` supplies the title when the stem has none.
fn parse_name(
    stem: &str,
    extension: Option<&str>,
    folder_season: Option<u16>,
    folder_title: impl Fn() -> Option<String>,
) -> ParsedName {
    if let Some(parsed) = russian(stem) {
        return ParsedName { title: parsed.title.or_else(folder_title), ..parsed };
    }
    if let Some(captures) = BARE_NUMBER.captures(stem)
        && let Ok(episode) = captures["episode"].parse()
    {
        return ParsedName {
            title: folder_title(),
            year: None,
            numbers: Numbers::Seasonless { episodes: vec![episode], folder_season },
            episode_title: captures.name("title").map(|title| words(title.as_str())),
        };
    }

    let file_name = match extension {
        Some(extension) => format!("{stem}.{extension}"),
        None => stem.to_owned(),
    };
    let result = hunch(&file_name);
    let date = result.date().and_then(|date| date.parse::<Date>().ok());
    let numbers = match date {
        Some(date) => Numbers::Date(date),
        None => numbers(&result, folder_season),
    };

    ParsedName {
        title: result.title().map(str::to_owned).or_else(folder_title),
        year: if date.is_some() { None } else { result.year().and_then(|year| i16::try_from(year).ok()) },
        numbers,
        episode_title: result.episode_title().map(str::to_owned),
    }
}

fn numbers(result: &HunchResult, folder_season: Option<u16>) -> Numbers {
    let mut episodes: Vec<u16> =
        result.all(Property::Episode).iter().filter_map(|episode| episode.parse().ok()).collect();
    if episodes.is_empty() {
        episodes = result.all(Property::AbsoluteEpisode).iter().filter_map(|episode| episode.parse().ok()).collect();
    }
    episodes.sort_unstable();
    episodes.dedup();

    if episodes.is_empty() {
        return Numbers::None;
    }
    match result.season().and_then(|season| u16::try_from(season).ok()) {
        Some(season) => Numbers::Episodes { season, episodes },
        None => Numbers::Seasonless { episodes, folder_season },
    }
}

fn russian(stem: &str) -> Option<ParsedName> {
    let captures = RUSSIAN.captures(stem)?;
    let season = captures.name("s1").or(captures.name("s2"))?.as_str().parse().ok()?;
    let episode = captures.name("e1").or(captures.name("e2"))?.as_str().parse().ok()?;
    let title = words(&captures["title"]);
    Some(ParsedName {
        title: (!title.is_empty()).then_some(title),
        year: None,
        numbers: Numbers::Episodes { season, episodes: vec![episode] },
        episode_title: None,
    })
}

fn folder_season(folder: &str) -> Option<u16> {
    if SPECIALS_FOLDER.is_match(folder.trim()) {
        return Some(0);
    }
    FOLDER_SEASON.iter().find_map(|pattern| pattern.captures(folder)?["season"].parse().ok())
}

/// Dots and underscores become spaces; separators at the ends are trimmed.
fn words(text: &str) -> String {
    let spaced = text.replace(['.', '_'], " ");
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '-' || c.is_whitespace())
        .to_owned()
}
