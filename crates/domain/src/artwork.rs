use serde::{Deserialize, Serialize};

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
