use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{process::Command, time::timeout};
use tracing::debug;
use yokoku_domain::Live;
use yokoku_media::{
    AudioStream, MediaInfo, SubtitleStream, VideoStream,
    ports::{MediaProbe, ProbeError},
};

const TIME_LIMIT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProbeSettings {
    /// `ffprobe` on the `PATH`, or a path to it.
    pub ffprobe: PathBuf,
}

impl Default for ProbeSettings {
    fn default() -> Self {
        Self { ffprobe: PathBuf::from("ffprobe") }
    }
}

/// Runs `ffprobe` on a file and reads its JSON report.
#[derive(Debug, Clone)]
pub struct FfProbe {
    program: Live<PathBuf>,
}

impl FfProbe {
    /// `program` is `ffprobe` on the `PATH`, or a path to it.
    pub fn new(program: Live<PathBuf>) -> Self {
        Self { program }
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
        let started = Instant::now();
        let run = Command::new(self.program.current())
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
        debug!(path = %path.display(), status = %output.status, elapsed_ms = started.elapsed().as_millis(), "ffprobe ran");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr.lines().find(|line| !line.trim().is_empty()).unwrap_or("it failed");
            return Err(failed(reason.trim().to_owned()));
        }
        let report: Report = serde_json::from_slice(&output.stdout).map_err(|error| failed(error.to_string()))?;
        Ok(report.into())
    }
}

impl From<Report> for MediaInfo {
    fn from(report: Report) -> Self {
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
                    info.audio.push(AudioStream { codec, language, channels: stream.channels.unwrap_or_default() });
                },
                Some("subtitle") => {
                    info.subtitles.push(SubtitleStream { codec, language, forced: stream.disposition.forced != 0 });
                },
                _ => {},
            }
        }
        info
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rstest::rstest;
    use yokoku_media::{AudioStream, MediaInfo, SubtitleStream, VideoStream};

    use super::Report;

    fn info(report: &str) -> MediaInfo {
        serde_json::from_str::<Report>(report).unwrap().into()
    }

    #[test]
    fn cover_art_is_not_the_video_and_undetermined_languages_are_unknown() {
        let info = info(
            r#"{ "streams": [
                { "codec_type": "video", "codec_name": "mjpeg", "width": 600, "height": 900, "disposition": { "attached_pic": 1 } },
                { "codec_type": "video", "codec_name": "hevc", "width": 1920, "height": 1080 },
                { "codec_type": "audio", "codec_name": "opus", "channels": 2, "tags": { "language": "und" } },
                { "codec_type": "attachment", "codec_name": "ttf" }
            ], "format": {} }"#,
        );

        assert_eq!(info.video, Some(VideoStream { codec: "hevc".into(), width: 1920, height: 1080 }));
        assert_eq!(info.audio, [AudioStream { codec: "opus".into(), language: None, channels: 2 }]);
        assert_eq!((info.duration, info.subtitles.len()), (None, 0));
    }

    #[test]
    fn forced_subtitles_are_marked() {
        let info = info(
            r#"{ "streams": [
                { "codec_type": "subtitle", "codec_name": "subrip", "tags": { "language": "rus" }, "disposition": { "forced": 1 } },
                { "codec_type": "subtitle", "codec_name": "ass" }
            ] }"#,
        );

        assert_eq!(
            info.subtitles,
            [
                SubtitleStream { codec: "subrip".into(), language: Some("rus".into()), forced: true },
                SubtitleStream { codec: "ass".into(), language: None, forced: false },
            ]
        );
    }

    #[rstest]
    #[case::seconds(r#""2.023""#, Some(Duration::from_millis(2023)))]
    #[case::negative(r#""-1""#, None)]
    #[case::not_a_number(r#""N/A""#, None)]
    fn the_duration_is_read_in_seconds(#[case] duration: &str, #[case] expected: Option<Duration>) {
        let info = info(&format!(r#"{{ "streams": [], "format": {{ "duration": {duration} }} }}"#));

        assert_eq!(info.duration, expected);
    }
}
