use dioxus::prelude::*;
use yokoku_domain::ItemName;

use crate::{api::add::SearchHit, components::skeleton::Skeleton, route::Route};

const POSTER: &str = "aspect-[2/3] w-28 shrink-0 border border-ink bg-subtle shadow-paper \
                      sm:h-[250px] sm:w-[170px] sm:aspect-auto";

/// A placeholder row while the results load.
#[component]
pub(super) fn HitSkeleton() -> Element {
    rsx! {
        li { class: "flex items-start gap-5 px-3 py-4",
            Skeleton { class: "aspect-[2/3] w-28 shrink-0 sm:h-[250px] sm:w-[170px] sm:aspect-auto" }
            div { class: "grid flex-1 gap-2",
                Skeleton { class: "h-6 w-1/3" }
                Skeleton { class: "h-4 w-full" }
                Skeleton { class: "h-4 w-5/6" }
                Skeleton { class: "h-4 w-2/3" }
            }
        }
    }
}

/// A search result to pick, or a link to its page when it is already in the library.
#[component]
pub(super) fn HitRow(hit: SearchHit, on_pick: Callback<SearchHit>) -> Element {
    let row = "flex w-full items-start gap-5 px-3 py-4 text-left hover:bg-subtle";
    let body = rsx! {
        match &hit.poster {
            Some(url) => rsx! {
                img {
                    class: "{POSTER} object-cover",
                    src: "{url}",
                    alt: "",
                    loading: "lazy",
                }
            },
            None => rsx! {
                span { class: POSTER }
            },
        }
        span { class: "grid min-w-0 flex-1 content-start gap-1.5",
            span { class: "text-section font-medium",
                "{hit.title}"
                if let Some(year) = ItemName::new(&hit.title, hit.year).shown_year() {
                    span { class: "font-normal text-muted", " ({year})" }
                }
            }
            if let Some(original) = &hit.original_title {
                span { class: "text-caption text-muted", "{original}" }
            }
            if !hit.overview.is_empty() {
                span { class: "mt-1 line-clamp-4 text-muted", "{hit.overview}" }
            }
        }
    };
    rsx! {
        li {
            match hit.in_library {
                Some(id) => rsx! {
                    Link { class: row, to: Route::item(id),
                        {body}
                        span { class: "shrink-0 text-caption text-success", "In library" }
                    }
                },
                None => rsx! {
                    button {
                        r#type: "button",
                        class: "{row} cursor-pointer focus-visible:outline-2 focus-visible:outline-ink",
                        onclick: move |_| on_pick(hit.clone()),
                        {body}
                    }
                },
            }
        }
    }
}
