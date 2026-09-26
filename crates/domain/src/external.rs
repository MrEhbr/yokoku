use std::{fmt, str::FromStr};

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
