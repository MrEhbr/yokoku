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

/// How an item is named to people: `Dune (2021)`, or the bare title without a year.
pub fn title_with_year(title: &str, year: Option<i16>) -> String {
    match year {
        Some(year) => format!("{title} ({year})"),
        None => title.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("Dune", Some(2021), "Dune (2021)")]
    #[case("Dune", None, "Dune")]
    fn names_an_item(#[case] title: &str, #[case] year: Option<i16>, #[case] expected: &str) {
        assert_eq!(title_with_year(title, year), expected);
    }
}
