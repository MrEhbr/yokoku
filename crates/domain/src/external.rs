use std::fmt;

/// An item's id at its metadata source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalId {
    Tmdb(u64),
    Tvdb(u64),
}

impl fmt::Display for ExternalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tmdb(id) => write!(f, "tmdb:{id}"),
            Self::Tvdb(id) => write!(f, "tvdb:{id}"),
        }
    }
}
