use std::{
    fmt,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

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

    pub fn series(self) -> Option<SeriesId> {
        match self {
            Self::Series(id) => Some(id),
            Self::Movie(_) => None,
        }
    }

    pub fn movie(self) -> Option<MovieId> {
        match self {
            Self::Series(_) => None,
            Self::Movie(id) => Some(id),
        }
    }
}

/// How an item is named to people: `Dune (2021)`, or the bare title without a year. A title that
/// already ends in its year, like `ONE PIECE (2023)`, is left as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemName<'a> {
    title: &'a str,
    year: Option<i16>,
}

impl<'a> ItemName<'a> {
    pub fn new(title: &'a str, year: Option<i16>) -> Self {
        Self { title, year }
    }

    /// The year, unless the title already ends in it.
    pub fn shown_year(&self) -> Option<i16> {
        self.year.filter(|year| !self.title.ends_with(&format!(" ({year})")))
    }
}

impl fmt::Display for ItemName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title)?;
        match self.shown_year() {
            Some(year) => write!(f, " ({year})"),
            None => Ok(()),
        }
    }
}

/// An item's id at its metadata source, written as `tmdb:1396` or `tvdb:81189`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalId {
    Tmdb(u64),
    Tvdb(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("expected a source id like tmdb:1396 or tvdb:81189, got {0:?}")]
pub struct ParseExternalIdError(String);

impl fmt::Display for ExternalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tmdb(id) => write!(f, "tmdb:{id}"),
            Self::Tvdb(id) => write!(f, "tvdb:{id}"),
        }
    }
}

impl FromStr for ExternalId {
    type Err = ParseExternalIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseExternalIdError(value.to_owned());
        let (provider, id) = value.split_once(':').ok_or_else(invalid)?;
        let id = id.parse().map_err(|_| invalid())?;
        match provider.to_ascii_lowercase().as_str() {
            "tmdb" => Ok(Self::Tmdb(id)),
            "tvdb" => Ok(Self::Tvdb(id)),
            _ => Err(invalid()),
        }
    }
}

/// What an item is about, as its metadata source describes it in the metadata language.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Description {
    /// Empty when the source has none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub overview: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    /// Minutes: a movie's length, or a series' usual episode length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<u16>,
}

/// An item's images as its metadata source gives them: TMDB paths such as `/abc.jpg`, or TVDB
/// URLs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Artwork {
    /// Portrait cover, 2:3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    /// Wide background, 16:9.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<String>,
    /// The title as a transparent image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkKind {
    Poster,
    Backdrop,
    Logo,
}

crate::string_enum!(ArtworkKind, "artwork kind" {
    Poster => "poster",
    Backdrop => "backdrop",
    Logo => "logo",
});

impl Artwork {
    pub fn get(&self, kind: ArtworkKind) -> Option<&str> {
        match kind {
            ArtworkKind::Poster => self.poster.as_deref(),
            ArtworkKind::Backdrop => self.backdrop.as_deref(),
            ArtworkKind::Logo => self.logo.as_deref(),
        }
    }
}

/// Where an item's files live: a folder named `name` directly in the root folder `root`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemFolder {
    pub root: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a folder name")]
pub struct InvalidFolderName(pub String);

impl ItemFolder {
    /// `name` must be a single path component.
    pub fn new(root: PathBuf, name: String) -> Result<Self, InvalidFolderName> {
        let mut components = Path::new(&name).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(part)), None) if part == name.as_str() => Ok(Self { root, name }),
            _ => Err(InvalidFolderName(name)),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(&self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Downloaded,
    Missing,
    Upcoming,
}

crate::string_enum!(FileStatus, "file status" {
    Downloaded => "downloaded",
    Missing => "missing",
    Upcoming => "upcoming",
});

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
        assert_eq!(ItemName::new(title, year).to_string(), expected);
    }
}
