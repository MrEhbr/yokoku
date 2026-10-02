use dioxus::prelude::*;

use crate::{api::library::detail::Description, format::runtime};

/// Genres, runtime and overview, each only when present.
/// A series' runtime is `per_episode`.
#[component]
pub fn ItemDescription(description: Description, #[props(default)] per_episode: bool) -> Element {
    let length = description.runtime.map(|minutes| {
        let length = runtime(u64::from(minutes));
        if per_episode { format!("{length} per episode") } else { length }
    });
    let facts: Vec<String> = description.genres.iter().cloned().chain(length).collect();
    rsx! {
        if !facts.is_empty() {
            p { class: "text-caption text-muted", {facts.join(" · ")} }
        }
        if !description.overview.is_empty() {
            p { class: "max-w-prose whitespace-pre-line", "{description.overview}" }
        }
    }
}
