use dioxus::prelude::*;
use dioxus_icons::lucide::Ellipsis;

use crate::{
    api::downloads::ItemLink,
    components::{
        button::{Button, ButtonVariant},
        dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
        refresh_button::{RefreshButton, use_refresh},
    },
    dialogs::{
        add_torrent::AddTorrentButton, remove::RemoveDialog, rename::RenameDialog,
        search_releases::SearchReleasesButton,
    },
};

/// An action the "More" menu holds below `sm`.
#[derive(Clone, Copy, PartialEq)]
enum More {
    Refresh,
    Rename,
    Remove,
}

/// The actions under an item's title: Add torrent and Search releases, then Refresh, Rename
/// files… and Remove…, which sit in a "More" menu below `sm`. `on_change` runs after the item
/// changes.
#[component]
pub fn ItemActions(item: ItemLink, on_change: Callback) -> Element {
    let id = item.id;
    let refresh = use_refresh(id, on_change);
    let mut renaming = use_signal(|| false);
    let mut removing = use_signal(|| false);
    let refreshing = (refresh.busy)();
    rsx! {
        div { class: "flex flex-wrap items-center gap-2",
            AddTorrentButton { item: item.clone() }
            SearchReleasesButton { item: item.clone() }
            div { class: "flex flex-wrap items-center gap-2 max-sm:hidden",
                RefreshButton { refresh }
                Button { onclick: move |_| renaming.set(true), "Rename files…" }
                Button { variant: ButtonVariant::Danger, onclick: move |_| removing.set(true), "Remove…" }
            }
            DropdownMenu { class: "sm:hidden",
                DropdownMenuTrigger {
                    class: "size-9 justify-center border border-control text-ink hover:not-disabled:bg-subtle [&>svg]:size-4",
                    aria_label: "More actions",
                    Ellipsis {}
                }
                DropdownMenuContent { class: "right-0 left-auto",
                    DropdownMenuItem::<More> {
                        value: More::Refresh,
                        index: 0usize,
                        disabled: refreshing,
                        on_select: move |_| {
                            spawn(refresh.run());
                        },
                        if refreshing {
                            "Refreshing…"
                        } else {
                            "Refresh"
                        }
                    }
                    DropdownMenuItem::<More> {
                        value: More::Rename,
                        index: 1usize,
                        on_select: move |_| renaming.set(true),
                        "Rename files…"
                    }
                    DropdownMenuItem::<More> {
                        class: "text-danger",
                        value: More::Remove,
                        index: 2usize,
                        on_select: move |_| removing.set(true),
                        "Remove…"
                    }
                }
            }
            if let Some(error) = (refresh.failed)() {
                span { role: "alert", class: "text-caption text-danger", "{error}" }
            }
        }
        RenameDialog { item: id, open: renaming, on_change }
        RemoveDialog {
            item: id,
            title: item.title,
            open: removing,
            on_change,
        }
    }
}
