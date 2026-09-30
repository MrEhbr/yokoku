use serde::{Deserialize, Serialize};
use yokoku_domain::Secret;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetadataSettings {
    /// Like `en-US`.
    pub language: String,
    /// Country whose movie release dates are used.
    pub region: String,
    pub tmdb: TmdbSettings,
    pub tvdb: TvdbSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TmdbSettings {
    /// API read access token.
    pub token: Option<Secret>,
    pub url: String,
}

/// Series come from TVDB while `api_key` is set.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TvdbSettings {
    /// Project API key.
    pub api_key: Option<Secret>,
    /// Subscriber PIN, needed with a user-supported API key.
    pub pin: Option<Secret>,
    pub url: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Unknown metadata language: {0}")]
pub struct UnknownLanguage(String);

impl MetadataSettings {
    /// `language` as the two-letter code TMDB images carry, e.g. `en` for `en-US`.
    pub fn image_language(&self) -> &str {
        self.language.split('-').next().unwrap_or_default()
    }

    /// `language` as the three-letter code TVDB takes, e.g. `eng` for `en-US`.
    pub fn tvdb_language(&self) -> Result<&'static str, UnknownLanguage> {
        isolang::Language::from_639_1(self.image_language())
            .map(|language| language.to_639_3())
            .ok_or_else(|| UnknownLanguage(self.language.clone()))
    }
}

impl Default for MetadataSettings {
    fn default() -> Self {
        Self {
            language: "en-US".into(),
            region: "US".into(),
            tmdb: TmdbSettings { token: None, url: "https://api.themoviedb.org/3".into() },
            tvdb: TvdbSettings { api_key: None, pin: None, url: "https://api4.thetvdb.com/v4".into() },
        }
    }
}
