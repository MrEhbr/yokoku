use std::path::{Path, PathBuf};

use crate::SubtitleTags;

/// `<video stem>[.<language>][.sdh][.forced].<extension>`, next to the video.
pub fn subtitle_path(video: &Path, tags: &SubtitleTags, extension: &str) -> PathBuf {
    let mut name = video.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let language = tags.language.as_deref().map(|language| clean(language, |c| c.is_ascii_alphanumeric() || c == '-'));
    if let Some(language) = language.filter(|language| !language.is_empty()) {
        name = format!("{name}.{language}");
    }
    if tags.sdh {
        name.push_str(".sdh");
    }
    if tags.forced {
        name.push_str(".forced");
    }
    let extension = clean(extension, |c| c.is_ascii_alphanumeric());
    if !extension.is_empty() {
        name = format!("{name}.{extension}");
    }
    video.with_file_name(name)
}

fn clean(text: &str, keep: impl Fn(char) -> bool) -> String {
    text.chars().filter(|&c| keep(c)).map(|c| c.to_ascii_lowercase()).collect()
}
