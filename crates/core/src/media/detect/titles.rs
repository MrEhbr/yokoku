use strsim::jaro_winkler;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};
use yokoku_domain::{Movie, Series};

use crate::media::detect::ParsedName;

const CLOSE_SIMILARITY: f64 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TitleMatch {
    Close,
    Exact,
}

/// Lowercase words without accents or punctuation; `&` reads as `and`.
pub(crate) fn normalize(title: &str) -> String {
    let unaccented: String = title.nfkd().filter(|&c| !is_combining_mark(c)).collect();
    let lowered = unaccented.to_lowercase().replace('&', " and ");
    lowered.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ")
}

/// The best match of `parsed` against any of an item's titles.
pub(crate) fn best_match<'a>(parsed: &str, titles: impl IntoIterator<Item = &'a str>) -> Option<TitleMatch> {
    let parsed = normalize(parsed);
    if parsed.is_empty() {
        return None;
    }
    titles.into_iter().filter_map(|title| compare(&parsed, &normalize(title))).max()
}

fn compare(parsed: &str, title: &str) -> Option<TitleMatch> {
    let extends = |text: &str, prefix: &str| text.strip_prefix(prefix).is_some_and(|rest| rest.starts_with(' '));
    if title.is_empty() {
        None
    } else if parsed.replace(' ', "") == title.replace(' ', "") {
        Some(TitleMatch::Exact)
    } else if extends(parsed, title) || extends(title, parsed) || jaro_winkler(parsed, title) >= CLOSE_SIMILARITY {
        Some(TitleMatch::Close)
    } else {
        None
    }
}

pub(crate) trait Titled {
    fn titles(&self) -> impl Iterator<Item = &str>;
    fn year(&self) -> Option<i16>;
}

impl Titled for Series {
    fn titles(&self) -> impl Iterator<Item = &str> {
        [&self.title, &self.original_title].into_iter().chain(&self.alternate_titles).map(String::as_str)
    }

    fn year(&self) -> Option<i16> {
        self.year
    }
}

impl Titled for Movie {
    fn titles(&self) -> impl Iterator<Item = &str> {
        [&self.title, &self.original_title].into_iter().chain(&self.alternate_titles).map(String::as_str)
    }

    fn year(&self) -> Option<i16> {
        self.year
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum YearFit {
    /// One year apart, e.g. a festival premiere against the release year.
    Near,
    Unknown,
    Exact,
}

impl YearFit {
    /// `None` when both years are known and more than one year apart.
    fn of(parsed: Option<i16>, item: Option<i16>) -> Option<Self> {
        match (parsed, item) {
            (Some(parsed), Some(item)) if parsed == item => Some(Self::Exact),
            (Some(parsed), Some(item)) if (i32::from(parsed) - i32::from(item)).abs() == 1 => Some(Self::Near),
            (Some(_), Some(_)) => None,
            _ => Some(Self::Unknown),
        }
    }
}

impl ParsedName {
    /// The single best title match; certain only for an exact title whose year agrees or is unknown (FR-4.6).
    pub(crate) fn choose<'a, T: Titled>(&self, items: &'a [T]) -> Option<(&'a T, bool)> {
        let title = self.title.as_deref()?;
        let scored: Vec<_> = items
            .iter()
            .filter_map(|item| {
                let fit = YearFit::of(self.year, item.year())?;
                Some((item, (best_match(title, item.titles())?, fit)))
            })
            .collect();

        let best = scored.iter().map(|(_, score)| *score).max()?;
        let mut top = scored.into_iter().filter(|(_, score)| *score == best);
        let (item, (title_match, fit)) = top.next()?;
        if top.next().is_some() {
            return None;
        }
        Some((item, title_match == TitleMatch::Exact && fit != YearFit::Near))
    }
}
