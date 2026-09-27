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
