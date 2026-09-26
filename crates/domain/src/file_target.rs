use serde::{Deserialize, Serialize};

use crate::{EpisodeSpan, MovieId, SeriesId};

/// What a video file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileTarget {
    Episodes { series: SeriesId, span: EpisodeSpan },
    Movie(MovieId),
}
