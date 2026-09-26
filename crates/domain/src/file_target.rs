use serde::{Deserialize, Serialize};

use crate::{EpisodeSpan, ItemId, MovieId, SeriesId};

/// What a video file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileTarget {
    Episodes { series: SeriesId, span: EpisodeSpan },
    Movie(MovieId),
}

impl FileTarget {
    /// Both hold the same movie or at least one common episode.
    pub fn overlaps(&self, other: &FileTarget) -> bool {
        match (self, other) {
            (Self::Episodes { series, span }, Self::Episodes { series: other_series, span: other_span }) => {
                series == other_series
                    && span.season() == other_span.season()
                    && span.first() <= other_span.last()
                    && other_span.first() <= span.last()
            },
            (Self::Movie(movie), Self::Movie(other_movie)) => movie == other_movie,
            _ => false,
        }
    }
}

impl FileTarget {
    /// The series or movie the file belongs to.
    pub fn item(&self) -> ItemId {
        match self {
            Self::Episodes { series, .. } => ItemId::Series(*series),
            Self::Movie(movie) => ItemId::Movie(*movie),
        }
    }
}
