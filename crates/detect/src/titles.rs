use strsim::jaro_winkler;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

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
    if title.is_empty() {
        None
    } else if parsed == title {
        Some(TitleMatch::Exact)
    } else if parsed.starts_with(&format!("{title} "))
        || title.starts_with(&format!("{parsed} "))
        || jaro_winkler(parsed, title) >= CLOSE_SIMILARITY
    {
        Some(TitleMatch::Close)
    } else {
        None
    }
}
