use dioxus::prelude::*;
use yokoku_web::components::ui::kbd::{Kbd, KbdGroup};

use crate::{Story, StoryPage};

#[component]
pub fn KbdStory() -> Element {
    rsx! {
        StoryPage {
            name: "Kbd",
            path: "ui::kbd",
            summary: "A keyboard key or shortcut.",
            Story { title: "Shortcut",
                KbdGroup {
                    Kbd { "Ctrl" }
                    Kbd { "K" }
                }
            }
        }
    }
}
