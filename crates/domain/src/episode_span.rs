use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EpisodeRef {
    pub season: u16,
    pub episode: u16,
}

impl fmt::Display for EpisodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S{:02}E{:02}", self.season, self.episode)
    }
}

/// Consecutive episodes of one season, as held by a multi-episode file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "SpanFields")]
pub struct EpisodeSpan {
    season: u16,
    first: u16,
    last: u16,
}

impl EpisodeSpan {
    /// `None` when `first` comes after `last`.
    pub fn new(season: u16, first: u16, last: u16) -> Option<Self> {
        (first <= last).then_some(Self { season, first, last })
    }

    pub fn single(reference: EpisodeRef) -> Self {
        Self { season: reference.season, first: reference.episode, last: reference.episode }
    }

    /// `None` unless the sorted `episodes` are non-empty and have no gaps.
    pub fn consecutive(season: u16, episodes: &[u16]) -> Option<Self> {
        let (&first, &last) = (episodes.first()?, episodes.last()?);
        if usize::from(last.checked_sub(first)?) + 1 != episodes.len() {
            return None;
        }
        Self::new(season, first, last)
    }

    pub fn season(&self) -> u16 {
        self.season
    }

    pub fn first(&self) -> u16 {
        self.first
    }

    pub fn last(&self) -> u16 {
        self.last
    }

    pub fn refs(&self) -> impl Iterator<Item = EpisodeRef> {
        let season = self.season;
        (self.first..=self.last).map(move |episode| EpisodeRef { season, episode })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("expected episodes like S01E02 or S01E01-E03, got {0:?}")]
pub struct ParseEpisodeSpanError(String);

impl FromStr for EpisodeSpan {
    type Err = ParseEpisodeSpanError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseEpisodeSpanError(value.to_owned());
        let number = |digits: &str| -> Result<u16, ParseEpisodeSpanError> {
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid());
            }
            digits.parse().map_err(|_| invalid())
        };

        let upper = value.to_ascii_uppercase();
        let (season, episodes) = upper.strip_prefix('S').and_then(|rest| rest.split_once('E')).ok_or_else(invalid)?;
        let (first, last) = episodes.split_once("-E").unwrap_or((episodes, episodes));
        Self::new(number(season)?, number(first)?, number(last)?).ok_or_else(invalid)
    }
}

#[derive(Deserialize)]
struct SpanFields {
    season: u16,
    first: u16,
    last: u16,
}

impl TryFrom<SpanFields> for EpisodeSpan {
    type Error = &'static str;

    fn try_from(fields: SpanFields) -> Result<Self, Self::Error> {
        Self::new(fields.season, fields.first, fields.last).ok_or("the first episode comes after the last")
    }
}

impl fmt::Display for EpisodeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", EpisodeRef { season: self.season, episode: self.first })?;
        if self.last != self.first {
            write!(f, "-E{:02}", self.last)?;
        }
        Ok(())
    }
}
