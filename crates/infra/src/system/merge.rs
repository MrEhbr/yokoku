use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::{process::Command, time::timeout};
use tracing::debug;
use yokoku_core::media::ports::{MergeError, Merger, Track};
use yokoku_domain::Live;

const PROBE_TIME_LIMIT: Duration = Duration::from_secs(60);
const MERGE_TIME_LIMIT: Duration = Duration::from_secs(30 * 60);

/// Merges with `ffmpeg`, copying the streams, after reading the tracks' streams with `ffprobe`.
#[derive(Debug, Clone)]
pub struct FfMpeg {
    ffmpeg: Live<PathBuf>,
    ffprobe: Live<PathBuf>,
}

impl FfMpeg {
    /// Each program is its name on the `PATH`, or a path to it.
    pub fn new(ffmpeg: Live<PathBuf>, ffprobe: Live<PathBuf>) -> Self {
        Self { ffmpeg, ffprobe }
    }

    /// The streams of `path`, in order.
    async fn streams(&self, path: &Path) -> Result<Vec<Stream>, MergeError> {
        let run = Command::new(self.ffprobe.current())
            .args(["-v", "error", "-print_format", "json", "-show_streams"])
            .arg(path)
            .kill_on_drop(true)
            .output();
        let output = run_within(PROBE_TIME_LIMIT, run, path).await?;
        let report: Report = serde_json::from_slice(&output.stdout)
            .map_err(|error| MergeError::Failed { path: path.to_owned(), reason: error.to_string() })?;
        Ok(report.streams)
    }
}

#[derive(Deserialize)]
struct Report {
    #[serde(default)]
    streams: Vec<Stream>,
}

#[derive(Deserialize)]
struct Stream {
    codec_type: Option<String>,
    #[serde(default)]
    tags: Tags,
}

#[derive(Default, Deserialize)]
struct Tags {
    language: Option<String>,
    title: Option<String>,
}

#[async_trait]
impl Merger for FfMpeg {
    async fn merge(&self, video: &Path, tracks: &[Track], to: &Path) -> Result<(), MergeError> {
        let failed = |reason: String| MergeError::Failed { path: to.to_owned(), reason };
        if let Some(folder) = to.parent() {
            tokio::fs::create_dir_all(folder).await.map_err(|error| failed(error.to_string()))?;
        }

        let mut inputs: Vec<OsString> = vec!["-i".into(), video.into()];
        let mut maps: Vec<OsString> = vec!["-map".into(), "0".into()];
        let mut tags: Vec<OsString> = Vec::new();
        let mut index = self.streams(video).await?.len();
        for (input, track) in (1..).zip(tracks) {
            inputs.extend(["-i".into(), track.path.clone().into()]);
            maps.extend(["-map".into(), input.to_string().into()]);
            for stream in self.streams(&track.path).await? {
                let missing =
                    |value: &Option<String>| value.as_deref().is_none_or(|value| value.is_empty() || value == "und");
                if let Some(language) = track.language.as_ref().filter(|_| missing(&stream.tags.language)) {
                    tags.extend([format!("-metadata:s:{index}").into(), format!("language={language}").into()]);
                }
                if let Some(title) = track.title.as_ref().filter(|_| missing(&stream.tags.title)) {
                    tags.extend([format!("-metadata:s:{index}").into(), format!("title={title}").into()]);
                }
                if track.forced && stream.codec_type.as_deref() == Some("subtitle") {
                    tags.extend([format!("-disposition:{index}").into(), "forced".into()]);
                }
                index += 1;
            }
        }

        let started = Instant::now();
        let run = Command::new(self.ffmpeg.current())
            .args(["-nostdin", "-v", "error", "-y"])
            .args(&inputs)
            .args(&maps)
            .args(["-c", "copy"])
            .args(&tags)
            .args(["-f", "matroska"])
            .arg(to)
            .kill_on_drop(true)
            .output();
        run_within(MERGE_TIME_LIMIT, run, to).await?;
        debug!(to = %to.display(), tracks = tracks.len(), elapsed_ms = started.elapsed().as_millis(), "ffmpeg merged");
        Ok(())
    }
}

/// The output of `run`, a program working on `path`, when it finishes successfully within `limit`.
async fn run_within(
    limit: Duration,
    run: impl Future<Output = io::Result<std::process::Output>>,
    path: &Path,
) -> Result<std::process::Output, MergeError> {
    let failed = |reason: String| MergeError::Failed { path: path.to_owned(), reason };
    let output = match timeout(limit, run).await {
        Err(_) => return Err(failed(format!("no answer within {} s", limit.as_secs()))),
        Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => return Err(MergeError::Missing),
        Ok(Err(error)) => return Err(failed(error.to_string())),
        Ok(Ok(output)) => output,
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.lines().find(|line| !line.trim().is_empty()).unwrap_or("it failed");
        return Err(failed(reason.trim().to_owned()));
    }
    Ok(output)
}
