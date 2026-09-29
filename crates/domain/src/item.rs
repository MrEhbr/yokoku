use serde::{Deserialize, Serialize};

use crate::{MovieId, SeriesId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Series,
    Movie,
}

crate::string_enum!(MediaKind, "media kind" {
    Series => "series",
    Movie => "movie",
});

/// A series or movie in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemId {
    Series(SeriesId),
    Movie(MovieId),
}

impl ItemId {
    pub fn kind(self) -> MediaKind {
        match self {
            Self::Series(_) => MediaKind::Series,
            Self::Movie(_) => MediaKind::Movie,
        }
    }
}

/// How an item is named to people: `Dune (2021)`, or the bare title without a year. A title that
/// already ends in its year, like `ONE PIECE (2023)`, is left as it is.
pub fn title_with_year(title: &str, year: Option<i16>) -> String {
    match year {
        Some(year) if !ends_with_year(title, year) => format!("{title} ({year})"),
        _ => title.to_owned(),
    }
}

/// Whether `title` ends in ` (year)`.
pub fn ends_with_year(title: &str, year: i16) -> bool {
    title.ends_with(&format!(" ({year})"))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("Dune", Some(2021), "Dune (2021)")]
    #[case("Dune", None, "Dune")]
    #[case("ONE PIECE (2023)", Some(2023), "ONE PIECE (2023)")]
    #[case("Dune (1984)", Some(2021), "Dune (1984) (2021)")]
    fn names_an_item(#[case] title: &str, #[case] year: Option<i16>, #[case] expected: &str) {
        assert_eq!(title_with_year(title, year), expected);
    }
}
