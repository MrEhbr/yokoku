use std::path::{Path, PathBuf};

use super::sanitize::{MAX_COMPONENT_BYTES, sanitize, truncate};

/// `<video stem>[.<suffix>].<extension>`, next to the video. Each dot-separated part of `suffix` is
/// made safe, and the suffix is shortened to keep the name within a path component's length.
pub fn sidecar_path(video: &Path, suffix: &str, extension: &str) -> PathBuf {
    let stem = video.file_stem().unwrap_or_default().to_string_lossy();
    let extension: String =
        extension.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
    let extension = if extension.is_empty() { String::new() } else { format!(".{extension}") };
    let suffix: Vec<String> = suffix.split('.').filter(|part| !part.trim().is_empty()).map(sanitize).collect();
    let suffix = if suffix.is_empty() { String::new() } else { format!(".{}", suffix.join(".")) };

    let room = MAX_COMPONENT_BYTES.saturating_sub(stem.len() + extension.len());
    video.with_file_name(format!("{stem}{}{extension}", truncate(&suffix, room).trim_end_matches('.')))
}
