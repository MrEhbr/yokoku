//! An entry's fields as both views render them.

use dioxus::prelude::*;

use crate::{
    api::library::Status,
    components::status::{self, Tone},
};

#[component]
pub(super) fn Lifecycle(status: Status) -> Element {
    let tone = match status {
        Status::Continuing | Status::Released => Tone::Success,
        Status::OnBreak | Status::Announced | Status::InCinemas => Tone::Info,
        Status::Ended => Tone::Muted,
    };
    rsx! {
        status::Status { tone, label: status.label() }
    }
}

#[component]
pub(super) fn Files(present: bool) -> Element {
    let (tone, label) = if present { (Tone::Success, "Files") } else { (Tone::Muted, "No files") };
    rsx! {
        status::Status { tone, label }
    }
}
