use dioxus::prelude::*;

use super::fields::{Files, Lifecycle};
use crate::{
    api::library::Entry,
    format::{date, year},
};

/// Poster cards; a title placeholder stands in for the artwork.
#[component]
pub(super) fn PosterGrid(entries: Vec<Entry>) -> Element {
    rsx! {
        ul { class: "grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-x-6 gap-y-8",
            for entry in entries {
                li { key: "{entry.id:?}", class: "flex flex-col",
                    div {
                        aria_hidden: "true",
                        class: "flex aspect-[2/3] items-end border border-ink bg-subtle p-3 shadow-paper",
                        span { class: "line-clamp-5 text-section font-medium break-words",
                            "{entry.title}"
                        }
                    }
                    h2 {
                        class: "mt-3 truncate font-medium leading-snug",
                        title: "{entry.title}",
                        "{entry.title}"
                    }
                    p { class: "mt-1 text-caption text-muted",
                        "{year(entry.year)} · {entry.status.kind().label()}"
                    }
                    div { class: "mt-2 flex flex-wrap gap-x-3 gap-y-1",
                        Lifecycle { status: entry.status }
                        Files { present: entry.has_files }
                    }
                    if let Some(release) = entry.next_release {
                        p { class: "mt-1 text-caption text-muted", "Next {date(release)}" }
                    }
                }
            }
        }
    }
}
