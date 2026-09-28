use dioxus::prelude::*;
use yokoku_web::components::ui::{
    button::Button,
    checkbox::{Checkbox, CheckboxState},
    label::Label,
    sheet::{Sheet, SheetContentClose, SheetDescription, SheetFooter, SheetHeader, SheetTitle},
};

use crate::{Story, StoryPage};

#[component]
pub fn SheetStory() -> Element {
    let mut open_right = use_signal(|| false);
    let mut open_left = use_signal(|| false);
    rsx! {
        StoryPage {
            name: "Sheet",
            path: "ui::sheet",
            summary: "A panel that slides in from an edge of the viewport, for filters and side content.",
            Story { title: "Filters from the right",
                Button { onclick: move |_| open_right.set(true), "Filters…" }
                Sheet {
                    open: Some(open_right()),
                    on_open_change: move |next| open_right.set(next),
                    SheetContentClose {}
                    SheetHeader {
                        SheetTitle { "Filters" }
                        SheetDescription { "Narrow the episodes shown below." }
                    }
                    div { class: "flex items-center gap-2 px-4",
                        Checkbox {
                            id: "sheet-only-missing",
                            default_checked: CheckboxState::Checked,
                        }
                        Label { html_for: "sheet-only-missing", "Only missing" }
                    }
                    SheetFooter {
                        Button { onclick: move |_| open_right.set(false), "Close" }
                    }
                }
            }
            Story { title: "From the left",
                Button { onclick: move |_| open_left.set(true), "Navigation…" }
                Sheet {
                    open: Some(open_left()),
                    on_open_change: move |next| open_left.set(next),
                    "data-side": "left",
                    SheetContentClose {}
                    SheetHeader {
                        SheetTitle { "Navigation" }
                        SheetDescription { "Overriding the default `data-side` attribute picks the edge." }
                    }
                }
            }
        }
    }
}
