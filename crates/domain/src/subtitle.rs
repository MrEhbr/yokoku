use std::fmt;

/// Jellyfin subtitle flags, e.g. from `Movie.en.sdh.forced.srt`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubtitleTags {
    /// Language code such as `en` or `pt-br`.
    pub language: Option<String>,
    /// Subtitles for the deaf and hard of hearing.
    pub sdh: bool,
    pub forced: bool,
}

/// The language with its flags, e.g. `en (SDH, forced)`; `unknown` without a language.
impl fmt::Display for SubtitleTags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.language.as_deref().unwrap_or("unknown"))?;
        match (self.sdh, self.forced) {
            (false, false) => Ok(()),
            (true, false) => f.write_str(" (SDH)"),
            (false, true) => f.write_str(" (forced)"),
            (true, true) => f.write_str(" (SDH, forced)"),
        }
    }
}
