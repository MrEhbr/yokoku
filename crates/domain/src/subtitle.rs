/// Jellyfin subtitle flags, e.g. from `Movie.en.sdh.forced.srt`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SubtitleTags {
    /// Language code such as `en` or `pt-br`.
    pub language: Option<String>,
    /// Subtitles for the deaf and hard of hearing.
    pub sdh: bool,
    pub forced: bool,
}
