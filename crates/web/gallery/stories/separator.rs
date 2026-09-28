use dioxus::prelude::*;
use yokoku_web::components::separator::Separator;

use crate::{Story, StoryPage};

#[component]
pub fn SeparatorStory() -> Element {
    rsx! {
        StoryPage {
            name: "Separator",
            path: "separator",
            summary: "A thin rule between groups of content.",
            Story { title: "Horizontal", Separator {} }
            Story { title: "Vertical",
                div { class: "flex h-6 items-center gap-3 text-caption",
                    "Movies"
                    Separator { horizontal: false }
                    "Series"
                }
            }
        }
    }
}
