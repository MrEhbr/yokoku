use dioxus::prelude::*;

use crate::dialogs::search_releases::SearchReleases;

/// Search configured indexers without starting from a library item.
#[component]
pub fn Releases() -> Element {
    rsx! {
        document::Title { "Search releases · Yokoku" }
        h1 { class: "yk-page-title", "Search releases" }
        p { class: "mt-2 text-muted", "Search Jackett and direct Torznab feeds, then choose a release to send to Transmission." }
        div { class: "mt-6", SearchReleases { item: None, on_close: None } }
    }
}
