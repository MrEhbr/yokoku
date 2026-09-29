use dioxus::prelude::*;

use super::fields::Files;
use crate::{
    api::library::Entry,
    components::item_status::Lifecycle,
    format::{date, year},
    route::Route,
};

/// Poster cards. The poster covers a title placeholder, which shows while the poster loads, when
/// it fails (its empty `alt` draws nothing), and for items without one. The title link covers
/// the whole card.
#[component]
pub(super) fn PosterGrid(entries: Vec<Entry>) -> Element {
    rsx! {
        ul { class: "grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-x-6 gap-y-8",
            for entry in entries {
                li { key: "{entry.id:?}", class: "relative flex flex-col",
                    div {
                        aria_hidden: "true",
                        class: "relative flex aspect-[2/3] items-end border border-ink bg-subtle p-3 shadow-paper",
                        span { class: "line-clamp-5 text-section font-medium break-words",
                            "{entry.title}"
                        }
                        if let Some(poster) = &entry.poster {
                            img {
                                class: "absolute inset-0 size-full object-cover",
                                src: "{poster}",
                                alt: "",
                                loading: "lazy",
                                decoding: "async",
                            }
                        }
                    }
                    h2 {
                        class: "mt-3 truncate font-medium leading-snug",
                        title: "{entry.title}",
                        Link {
                            class: "after:absolute after:inset-0 hover:underline",
                            to: Route::item(entry.id),
                            "{entry.title}"
                        }
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
