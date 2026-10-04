use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::TempDir;
use yokoku_core::media::ports::{MergeError, Merger, Track};
use yokoku_domain::Live;
use yokoku_infra::system::FfMpeg;

fn ffmpeg(args: &[&str], output: &Path) {
    let status = Command::new("ffmpeg").args(["-v", "error", "-y"]).args(args).arg(output).status().unwrap();
    assert!(status.success());
}

/// Each stream of `path` as `type language title forced`.
fn streams(path: &Path) -> Vec<String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type:stream_tags=language,title:stream_disposition=forced",
        ])
        .args(["-of", "json"])
        .arg(path)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    report["streams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|stream| {
            let tag = |name: &str| stream["tags"][name].as_str().unwrap_or("-").to_owned();
            format!(
                "{} {} {} {}",
                stream["codec_type"].as_str().unwrap(),
                tag("language"),
                tag("title"),
                stream["disposition"]["forced"]
            )
        })
        .collect()
}

#[tokio::test]
#[ignore = "needs ffmpeg and ffprobe on PATH"]
async fn real_ffmpeg_merges_tracks_and_fills_in_only_missing_tags() {
    let dir = TempDir::new().unwrap();
    let video = dir.path().join("video.mkv");
    ffmpeg(
        &["-f", "lavfi", "-i", "testsrc=size=320x180:rate=10:duration=1", "-f", "lavfi", "-i", "sine=duration=1"],
        &video,
    );
    let dub = dir.path().join("dub.mka");
    ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=1",
            "-metadata:s:a:0",
            "language=rus",
            "-metadata:s:a:0",
            "title=Studio",
        ],
        &dub,
    );
    let subtitle = dir.path().join("signs.srt");
    fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:01,000\nHello\n").unwrap();
    let to = dir.path().join("Season 01/merged.mkv");
    let tracks = [
        Track { path: dub, language: Some("eng".into()), title: Some("Ignored".into()), forced: false },
        Track { path: subtitle, language: Some("rus".into()), title: Some("Subs Group".into()), forced: true },
    ];

    FfMpeg::new(Live::fixed("ffmpeg".into()), Live::fixed("ffprobe".into())).merge(&video, &tracks, &to).await.unwrap();

    assert_eq!(streams(&to), ["video - - 0", "audio - - 0", "audio rus Studio 0", "subtitle rus Subs Group 1"]);
}

#[tokio::test]
async fn a_missing_ffprobe_or_ffmpeg_is_reported() {
    let dir = TempDir::new().unwrap();
    let video = dir.path().join("video.mkv");
    fs::write(&video, b"video").unwrap();
    let missing = || Live::fixed(PathBuf::from("/nonexistent/program"));

    let error = FfMpeg::new(missing(), missing()).merge(&video, &[], &dir.path().join("out.mkv")).await.unwrap_err();

    assert!(matches!(error, MergeError::Missing), "{error}");
}
