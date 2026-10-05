use dioxus::prelude::*;
use dioxus_icons::lucide::RefreshCw;
use yokoku_domain::ItemId;

use crate::{
    api::{failure, library::manage::refresh},
    components::button::Button,
};

/// Reading an item's metadata again, shared by the controls that start it: whether it runs, and
/// why it last failed.
#[derive(Clone, Copy, PartialEq)]
pub struct Refresh {
    item: ItemId,
    on_change: Callback,
    pub busy: Signal<bool>,
    pub failed: Signal<Option<String>>,
}

/// `on_change` runs once a refresh of `item` is saved.
pub fn use_refresh(item: ItemId, on_change: Callback) -> Refresh {
    Refresh { item, on_change, busy: use_signal(|| false), failed: use_signal(|| None) }
}

impl Refresh {
    /// Reads the item's metadata from its source again.
    pub async fn run(mut self) {
        self.busy.set(true);
        self.failed.set(None);
        match refresh(self.item).await {
            Ok(()) => self.on_change.call(()),
            Err(error) => self.failed.set(Some(failure(&error))),
        }
        self.busy.set(false);
    }
}

/// Starts `refresh`; its failure shows where `refresh.failed` is rendered.
#[component]
pub fn RefreshButton(refresh: Refresh) -> Element {
    let busy = (refresh.busy)();
    rsx! {
        Button {
            disabled: busy,
            aria_busy: busy,
            title: "Read titles, dates, episodes and artwork from the metadata source again",
            onclick: move |_| refresh.run(),
            RefreshCw {
                size: "1rem",
                class: if busy { "animate-spin motion-reduce:animate-none" } else { "" },
            }
            "Refresh"
        }
    }
}
