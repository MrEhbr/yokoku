use dioxus::prelude::*;
use yokoku_web::components::radio_group::{RadioGroup, RadioItem};

use crate::{Story, StoryPage};

#[component]
pub fn RadioGroupStory() -> Element {
    let mut mode = use_signal(|| Some("hardlink".to_string()));
    rsx! {
        StoryPage {
            name: "Radio group",
            path: "radio_group",
            summary: "One choice from a short list of options, such as an import mode.",
            Story { title: "Import mode",
                div { class: "flex flex-col gap-3",
                    RadioGroup {
                        aria_label: "Import mode",
                        value: mode,
                        on_value_change: move |next| mode.set(Some(next)),
                        RadioItem { value: "hardlink".to_string(), index: 0usize, "Hard link" }
                        RadioItem { value: "copy".to_string(), index: 1usize, "Copy" }
                        RadioItem {
                            value: "move".to_string(),
                            index: 2usize,
                            disabled: true,
                            "Move (disabled)"
                        }
                    }
                    p { class: "text-caption text-muted", "Mode: {mode().unwrap_or_default()}" }
                }
            }
        }
    }
}
