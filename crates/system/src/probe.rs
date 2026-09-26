use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::{process::Command, time::timeout};
use yokoku_media::{
    AudioStream, MediaInfo, SubtitleStream, VideoStream,
    ports::{MediaProbe, ProbeError},
};

const TIME_LIMIT: Duration = Duration::from_secs(60);

/// Runs `ffprobe` on a file and reads its JSON report.
#[derive(Debug, Clone)]
pub struct FfProbe {
    program: PathBuf,
}

impl FfProbe {
    /// `program` is `ffprobe` on the `PATH`, or a path to it.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self { program: program.into() }
    }
}

#[derive(Deserialize)]
struct Report {
    #[serde(default)]
    streams: Vec<Stream>,
    format: Option<Format>,
}

#[derive(Deserialize)]
struct Format {
    /// Seconds, like `"2.023000"`.
    duration: Option<String>,
}

#[derive(Deserialize)]
struct Stream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    channels: Option<u16>,
    #[serde(default)]
    tags: Tags,
    #[serde(default)]
    disposition: Disposition,
}

#[derive(Default, Deserialize)]
struct Tags {
    language: Option<String>,
}

#[derive(Default, Deserialize)]
struct Disposition {
    #[serde(default)]
    forced: u8,
    #[serde(default)]
    attached_pic: u8,
}

#[async_trait]
impl MediaProbe for FfProbe {
    async fn probe(&self, path: &Path) -> Result<MediaInfo, ProbeError> {
        let failed = |reason: String| ProbeError::Failed { path: path.to_owned(), reason };
        let run = Command::new(&self.program)
            .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
            .arg(path)
            .kill_on_drop(true)
            .output();
        let output = match timeout(TIME_LIMIT, run).await {
            Err(_) => return Err(failed(format!("no answer within {} s", TIME_LIMIT.as_secs()))),
            Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => return Err(ProbeError::Missing),
            Ok(Err(error)) => return Err(failed(error.to_string())),
            Ok(Ok(output)) => output,
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr.lines().find(|line| !line.trim().is_empty()).unwrap_or("it failed");
            return Err(failed(reason.trim().to_owned()));
        }
        let report: Report = serde_json::from_slice(&output.stdout).map_err(|error| failed(error.to_string()))?;
        Ok(media_info(report))
    }
}

fn media_info(report: Report) -> MediaInfo {
    let mut info = MediaInfo {
        duration: report
            .format
            .and_then(|format| format.duration?.parse::<f64>().ok())
            .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
            .map(Duration::from_secs_f64),
        ..MediaInfo::default()
    };
    for stream in report.streams {
        let codec = stream.codec_name.unwrap_or_default();
        let language = stream.tags.language.filter(|language| language != "und");
        match stream.codec_type.as_deref() {
            Some("video") if stream.disposition.attached_pic == 0 && info.video.is_none() => {
                if let (Some(width), Some(height)) = (stream.width, stream.height) {
                    info.video = Some(VideoStream { codec, width, height });
                }
            },
            Some("audio") => {
                info.audio.push(AudioStream { codec, language, channels: stream.channels.unwrap_or_default() })
            },
            Some("subtitle") => {
                info.subtitles.push(SubtitleStream { codec, language, forced: stream.disposition.forced != 0 })
            },
            _ => {},
        }
    }
    info
}
