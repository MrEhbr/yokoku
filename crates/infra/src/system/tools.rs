use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the media programs are: each its name on the `PATH`, or a path to it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MediaTools {
    /// Reads the streams of files.
    pub ffprobe: PathBuf,
    /// Merges external tracks into videos.
    pub ffmpeg: PathBuf,
}

impl Default for MediaTools {
    fn default() -> Self {
        Self { ffprobe: PathBuf::from("ffprobe"), ffmpeg: PathBuf::from("ffmpeg") }
    }
}
