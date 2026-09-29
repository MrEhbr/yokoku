use serde::{Deserialize, Serialize};

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
