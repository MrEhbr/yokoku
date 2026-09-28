use dioxus::prelude::*;
use yokoku_web::components::ui::{label::Label, switch::Switch};

use crate::{Story, StoryPage};

#[component]
pub fn SwitchStory() -> Element {
    rsx! {
        StoryPage {
            name: "Switch",
            path: "ui::switch",
            summary: "An on/off setting that applies immediately, without a separate save action.",
            Story { title: "States",
                div { class: "flex flex-col gap-3",
                    div { class: "flex items-center gap-2",
                        Switch { id: "auto", default_checked: true }
                        Label { html_for: "auto", "Import certain matches" }
                    }
                    div { class: "flex items-center gap-2",
                        Switch { id: "subs" }
                        Label { html_for: "subs", "Rename subtitles" }
                    }
                    div { class: "flex items-center gap-2",
                        Switch { id: "locked-switch", disabled: true }
                        Label { html_for: "locked-switch", "Disabled" }
                    }
                }
            }
        }
    }
}
