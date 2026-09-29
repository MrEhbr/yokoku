use dioxus::prelude::*;

use crate::components::history_list::{HistoryList, HistoryScope};

/// What happened in the library, newest first (FR-9.1).
#[component]
pub fn History() -> Element {
    rsx! {
        document::Title { "History · Yokoku" }
        h1 { class: "yk-page-title", "History" }
        p { class: "mt-2 text-muted", "Items added and removed, downloads, imports and file changes." }
        div { class: "mt-8",
            HistoryList { scope: HistoryScope::Library, empty: "Nothing has happened yet." }
        }
    }
}
