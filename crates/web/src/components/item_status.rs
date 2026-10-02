//! Library statuses, one component per category of the status taxonomy.

use dioxus::prelude::*;

use super::{
    status::{Status, Tone},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use crate::{
    api::library::{self, FileStatus, detail::Watched},
    format::date,
};

/// Series or movie lifecycle: continuing, on break, ended; announced, in cinemas, released.
#[component]
pub fn Lifecycle(status: library::Status) -> Element {
    let tone = match status {
        library::Status::Continuing | library::Status::Released => Tone::Success,
        library::Status::OnBreak | library::Status::Announced | library::Status::InCinemas => Tone::Info,
        library::Status::Ended => Tone::Muted,
    };
    rsx! {
        Status { tone, label: status.label() }
    }
}

/// An episode's or movie's file: downloaded, missing, or upcoming, which an `episode` calls
/// "Not yet aired". Only a `monitored` item is missing; otherwise it has "No file".
#[component]
pub fn FileState(
    status: FileStatus,
    #[props(default)] episode: bool,
    #[props(default = true)] monitored: bool,
) -> Element {
    let (tone, label) = match status {
        FileStatus::Downloaded => (Tone::Success, status.label()),
        FileStatus::Missing if !monitored => (Tone::Muted, "No file"),
        FileStatus::Missing => (Tone::Warning, status.label()),
        FileStatus::Upcoming if episode => (Tone::Info, "Not yet aired"),
        FileStatus::Upcoming => (Tone::Info, status.label()),
    };
    rsx! {
        Status { tone, label }
    }
}

/// Whether the Jellyfin user has played a downloaded file; the date, when known, shows on hover.
#[component]
pub fn FileWatched(watched: Option<Watched>) -> Element {
    let (tone, label) = match watched {
        Some(_) => (Tone::Success, "Watched".to_owned()),
        None => (Tone::Muted, "Unwatched".to_owned()),
    };
    let played = watched.and_then(|watched| watched.on).map(|on| format!("Last played {}", date(on)));
    rsx! {
        if let Some(played) = played {
            Tooltip { class: "inline-flex",
                TooltipTrigger {
                    Status { tone, label }
                }
                TooltipContent { "{played}" }
            }
        } else {
            Status { tone, label }
        }
    }
}

/// How many of a series' downloaded episodes the Jellyfin user has played.
#[component]
pub fn SeriesWatched(watched: usize, downloaded: usize) -> Element {
    let (tone, label) = match watched {
        0 => (Tone::Muted, "Unwatched".to_owned()),
        all if all == downloaded => (Tone::Success, "Watched".to_owned()),
        some => (Tone::Info, format!("In progress · {some} of {downloaded} watched")),
    };
    rsx! {
        Status { tone, label }
    }
}
