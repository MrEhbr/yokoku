//! A library file as probed: a short summary and the full details.

use dioxus::prelude::*;

use crate::{
    api::library::detail::FileInfo,
    format::{resolution, runtime, size},
};

/// `1080p · 1.4 GB`, or the size alone until the file is probed.
#[component]
pub fn FileSummary(info: FileInfo) -> Element {
    let picture = info.streams.and_then(|streams| streams.video).map(|video| resolution(video.width, video.height));
    let summary: Vec<String> = picture.into_iter().chain([size(info.size)]).collect();
    rsx! {
        span { class: "text-caption text-muted", {summary.join(" · ")} }
    }
}

/// Path, size, length, picture, audio and subtitles, each on its own row.
#[component]
pub fn FileDetails(info: FileInfo) -> Element {
    let mut rows: Vec<(&str, String)> = vec![("Size", size(info.size))];
    match &info.streams {
        Some(streams) => {
            rows.extend(streams.minutes.map(|minutes| ("Length", runtime(minutes))));
            rows.extend(streams.video.as_ref().map(|video| {
                let picture = resolution(video.width, video.height);
                ("Video", format!("{picture} · {}x{} {}", video.width, video.height, video.codec))
            }));
            if !streams.audio.is_empty() {
                rows.push(("Audio", streams.audio.join(", ")));
            }
            let places = [(&streams.subtitles, "in the file"), (&info.subtitle_files, "beside it")];
            let subtitles: Vec<String> = places
                .into_iter()
                .filter(|(languages, _)| !languages.is_empty())
                .map(|(languages, place)| format!("{} {place}", languages.join(", ")))
                .collect();
            if !subtitles.is_empty() {
                rows.push(("Subtitles", subtitles.join("; ")));
            }
        },
        None => rows.push(("Streams", "Not read yet; run `yokoku files probe`".to_owned())),
    }
    rsx! {
        dl { class: "grid gap-x-6 gap-y-1 text-caption sm:grid-cols-[auto_1fr]",
            dt { class: "text-muted", "Path" }
            dd { class: "yk-code break-all", "{info.path}" }
            for (label, value) in rows {
                div { key: "{label}", class: "contents",
                    dt { class: "text-muted", "{label}" }
                    dd { "{value}" }
                }
            }
        }
    }
}
