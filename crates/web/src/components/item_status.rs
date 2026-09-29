//! Library statuses, one component per category of the status taxonomy.

use dioxus::prelude::*;

use super::status::{Status, Tone};
use crate::api::library::{self, FileStatus};

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

#[component]
pub fn Monitoring(monitored: bool) -> Element {
    let (tone, label) = if monitored { (Tone::Success, "Monitored") } else { (Tone::Muted, "Unmonitored") };
    rsx! {
        Status { tone, label }
    }
}
