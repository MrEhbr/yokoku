//! An entry's fields as both views render them.

use dioxus::prelude::*;

use crate::components::status::{self, Tone};

#[component]
pub(super) fn Files(present: bool) -> Element {
    let (tone, label) = if present { (Tone::Success, "Files") } else { (Tone::Muted, "No files") };
    rsx! {
        status::Status { tone, label }
    }
}
