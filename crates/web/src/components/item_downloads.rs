//! A library item's torrents and their current status.

use dioxus::prelude::*;

use crate::{
    api::downloads::{DownloadEntry, DownloadState, ImportState},
    components::{
        progress::Progress,
        status::{Status, Tone},
    },
    route::Route,
};

#[component]
pub fn ItemDownloads(downloads: Vec<DownloadEntry>) -> Element {
    rsx! {
        section { class: "flex flex-col gap-2",
            h2 { class: "text-caption font-medium text-muted", "Downloads" }
            for download in downloads {
                div {
                    key: "{download.id}",
                    class: "flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-caption",
                    span { class: "min-w-0 [overflow-wrap:anywhere]", "{download.name}" }
                    if let Some(season) = download.season {
                        span { class: "text-muted",
                            if season == 0 {
                                "Specials"
                            } else {
                                "Season {season}"
                            }
                        }
                    }
                    Status { tone: tone(&download), label: label(&download) }
                    Link {
                        class: "text-muted underline-offset-4 hover:text-ink hover:underline",
                        to: Route::Downloads {},
                        "View in Queue"
                    }
                    if download.state == DownloadState::Downloading {
                        div { class: "w-full max-w-64",
                            Progress {
                                value: f64::from(download.percent),
                                aria_label: "{download.name} progress",
                            }
                        }
                    }
                }
            }
        }
    }
}

fn tone(download: &DownloadEntry) -> Tone {
    match (&download.state, download.import.as_ref().map(|import| &import.state)) {
        (DownloadState::Error(_), _) | (_, Some(ImportState::Failed(_))) => Tone::Danger,
        (_, Some(ImportState::NeedsReview)) => Tone::Warning,
        (_, Some(ImportState::Queued | ImportState::Importing)) => Tone::Info,
        (_, _) if download.imported => Tone::Success,
        (DownloadState::Queued | DownloadState::Checking | DownloadState::Downloading, _) => Tone::Info,
        _ => Tone::Muted,
    }
}

fn label(download: &DownloadEntry) -> String {
    match (&download.state, download.import.as_ref().map(|import| &import.state)) {
        (DownloadState::Error(_), _) => "Error".to_owned(),
        (_, Some(ImportState::NeedsReview)) => "Needs review".to_owned(),
        (_, Some(ImportState::Failed(_))) => "Import failed".to_owned(),
        (_, Some(ImportState::Queued)) => "Waiting to import".to_owned(),
        (_, Some(ImportState::Importing)) => "Importing…".to_owned(),
        (_, _) if download.imported => "Imported".to_owned(),
        (DownloadState::Downloading, _) => format!("Downloading · {}%", download.percent),
        (DownloadState::Paused, _) => format!("Paused at {}%", download.percent),
        (state, _) => state.label().to_owned(),
    }
}
