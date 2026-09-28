use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use yokoku_web::components::dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger};

use crate::{Story, StoryPage};

#[component]
pub fn DropdownMenuStory() -> Element {
    rsx! {
        StoryPage {
            name: "Dropdown menu",
            path: "dropdown_menu",
            summary: "A menu of actions opened from a trigger. Items apply only to the current selection.",
            Story { title: "Bulk actions",
                div { class: "h-48",
                    DropdownMenu {
                        DropdownMenuTrigger { class: "min-h-9 gap-2 border border-control px-3 text-body font-medium text-ink \
                                    hover:not-disabled:bg-subtle",
                            "Bulk actions (3)"
                            ChevronDown {
                                size: "1rem",
                                class: "transition-transform group-data-[state=open]:rotate-180",
                            }
                        }
                        DropdownMenuContent {
                            DropdownMenuItem::<&'static str> { value: "set-series", index: 0usize, "Set series" }
                            DropdownMenuItem::<&'static str> { value: "set-season", index: 1usize, "Set season" }
                            DropdownMenuItem::<&'static str> {
                                value: "remove",
                                index: 2usize,
                                class: "text-danger",
                                "Remove from queue"
                            }
                        }
                    }
                }
            }
        }
    }
}
