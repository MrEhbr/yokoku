use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// A tracker's id at the indexer, like `rutor`: ASCII letters, digits, `-` and `_`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TrackerId(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("expected a tracker id of letters, digits, - and _, got {0:?}")]
pub struct InvalidTrackerId(String);

impl TrackerId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for TrackerId {
    type Err = InvalidTrackerId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_owned())
    }
}

impl TryFrom<String> for TrackerId {
    type Error = InvalidTrackerId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if value.is_empty() || !value.chars().all(plain) {
            return Err(InvalidTrackerId(value));
        }
        Ok(Self(value))
    }
}

impl From<TrackerId> for String {
    fn from(id: TrackerId) -> Self {
        id.0
    }
}

impl fmt::Display for TrackerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One tracker or more, each once, in the order first given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<TrackerId>", into = "Vec<TrackerId>")]
pub struct TrackerSet(Vec<TrackerId>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("expected at least one tracker")]
pub struct EmptyTrackerSet;

impl TrackerSet {
    /// `None` without a tracker.
    pub fn new(ids: impl IntoIterator<Item = TrackerId>) -> Option<Self> {
        let mut unique: Vec<TrackerId> = Vec::new();
        for id in ids {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        (!unique.is_empty()).then_some(Self(unique))
    }

    pub fn ids(&self) -> &[TrackerId] {
        &self.0
    }
}

impl TryFrom<Vec<TrackerId>> for TrackerSet {
    type Error = EmptyTrackerSet;

    fn try_from(ids: Vec<TrackerId>) -> Result<Self, Self::Error> {
        Self::new(ids).ok_or(EmptyTrackerSet)
    }
}

impl From<TrackerSet> for Vec<TrackerId> {
    fn from(set: TrackerSet) -> Self {
        set.0
    }
}

/// The trackers a search covers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trackers {
    /// Every tracker the indexer has.
    #[default]
    All,
    Only(TrackerSet),
}
