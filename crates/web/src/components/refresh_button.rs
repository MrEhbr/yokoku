use dioxus::prelude::*;
use dioxus_icons::lucide::RefreshCw;
use yokoku_domain::ItemId;

use crate::{
    api::{failure, library::manage::refresh},
    components::button::Button,
};

/// Reads the item's metadata from its source again and calls `on_change` once it is saved.
#[component]
pub fn RefreshButton(item: ItemId, on_change: Callback) -> Element {
    let mut busy = use_signal(|| false);
    let mut failed = use_signal(|| None::<String>);
    rsx! {
        span { class: "inline-flex items-center gap-2",
            Button {
                disabled: busy(),
                aria_busy: busy(),
                title: "Read titles, dates, episodes and artwork from the metadata source again",
                onclick: move |_| async move {
                    busy.set(true);
                    failed.set(None);
                    match refresh(item).await {
                        Ok(()) => on_change(()),
                        Err(error) => failed.set(Some(failure(&error))),
                    }
                    busy.set(false);
                },
                RefreshCw {
                    size: "1rem",
                    class: if busy() { "animate-spin motion-reduce:animate-none" } else { "" },
                }
                "Refresh"
            }
            if let Some(error) = failed() {
                span { role: "alert", class: "text-caption text-danger", "{error}" }
            }
        }
    }
}
