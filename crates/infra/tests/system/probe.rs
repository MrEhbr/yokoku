use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use tempfile::TempDir;
use yokoku_core::media::{
    AudioStream, MediaInfo, SubtitleStream, VideoStream,
    ports::{MediaProbe, ProbeError},
};
use yokoku_domain::Live;
use yokoku_infra::system::FfProbe;

/// A stand-in for `ffprobe` that prints `stdout` and `stderr` and exits with `code`.
fn stand_in(dir: &Path, stdout: &str, stderr: &str, code: i32) -> PathBuf {
    fs::write(dir.join("report.json"), stdout).unwrap();
    fs::write(dir.join("error.txt"), stderr).unwrap();
    let program = dir.join("ffprobe");
    let script = format!("#!/bin/sh\ncat '{0}/report.json'\ncat '{0}/error.txt' >&2\nexit {code}\n", dir.display());
    fs::write(&program, script).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

fn sample() -> MediaInfo {
    MediaInfo {
        duration: Some(Duration::from_millis(2023)),
        video: Some(VideoStream { codec: "h264".into(), width: 320, height: 180 }),
        audio: vec![
            AudioStream { codec: "aac".into(), language: Some("eng".into()), channels: 2 },
            AudioStream { codec: "aac".into(), language: Some("jpn".into()), channels: 1 },
        ],
        subtitles: vec![SubtitleStream { codec: "subrip".into(), language: Some("rus".into()), forced: true }],
    }
}

#[tokio::test]
async fn reads_video_audio_and_subtitle_streams() {
    let dir = TempDir::new().unwrap();
    let report = include_str!("fixtures/ffprobe_sample.json");
    let program = stand_in(dir.path(), report, "", 0);

    let info = FfProbe::new(Live::fixed(program)).probe(Path::new("/movies/sample.mkv")).await.unwrap();

    assert_eq!(info, sample());
}

#[tokio::test]
async fn a_file_it_cannot_read_fails_with_its_first_error_line() {
    let dir = TempDir::new().unwrap();
    let program = stand_in(dir.path(), "", "\n/tv/a.mkv: Invalid data found when processing input\n", 1);

    let error = FfProbe::new(Live::fixed(program)).probe(Path::new("/tv/a.mkv")).await.unwrap_err();

    assert!(
        matches!(&error, ProbeError::Failed { path, reason }
            if path == Path::new("/tv/a.mkv") && reason == "/tv/a.mkv: Invalid data found when processing input"),
        "{error}"
    );
}

#[tokio::test]
async fn a_missing_program_is_reported_as_missing() {
    let error =
        FfProbe::new(Live::fixed("/nonexistent/ffprobe".into())).probe(Path::new("/tv/a.mkv")).await.unwrap_err();

    assert!(matches!(error, ProbeError::Missing), "{error}");
}

#[tokio::test]
#[ignore = "needs ffmpeg and ffprobe on PATH"]
async fn real_ffprobe_reads_a_generated_file() {
    let dir = TempDir::new().unwrap();
    let subtitle = dir.path().join("sub.srt");
    fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:01,000\nHello\n").unwrap();
    let video = dir.path().join("sample.mkv");
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "testsrc=size=320x180:rate=10:duration=2"])
        .args(["-f", "lavfi", "-i", "sine=duration=2", "-f", "lavfi", "-i", "sine=frequency=880:duration=2"])
        .arg("-i")
        .arg(&subtitle)
        .args(["-map", "0", "-map", "1", "-map", "2", "-map", "3", "-c:v", "libx264", "-c:a", "aac"])
        .args(["-ac:a:0", "2", "-ac:a:1", "1", "-c:s", "srt", "-metadata:s:a:0", "language=eng"])
        .args(["-metadata:s:a:1", "language=jpn", "-metadata:s:s:0", "language=rus", "-disposition:s:0", "forced"])
        .arg(&video)
        .status()
        .unwrap();
    assert!(status.success());

    let info = FfProbe::new(Live::fixed("ffprobe".into())).probe(&video).await.unwrap();

    assert_eq!(MediaInfo { duration: None, ..info.clone() }, MediaInfo { duration: None, ..sample() });
    assert!(info.duration.is_some_and(|duration| duration >= Duration::from_secs(2)));
}
