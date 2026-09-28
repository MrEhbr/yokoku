use dioxus::prelude::*;
use yokoku_web::components::ui::hover_card::{HoverCard, HoverCardContent, HoverCardTrigger};

use crate::{Story, StoryPage};

#[component]
pub fn HoverCardStory() -> Element {
    rsx! {
        StoryPage {
            name: "Hover card",
            path: "ui::hover_card",
            summary: "Supplementary details shown while hovering or focusing a trigger. Keep essentials elsewhere for touch.",
            Story { title: "Series preview (hover or focus the title)",
                div { class: "h-32",
                    HoverCard {
                        HoverCardTrigger {
                            span { class: "underline decoration-dotted", "The Expanse" }
                        }
                        HoverCardContent {
                            p { class: "font-medium", "The Expanse (2015)" }
                            p { class: "text-caption text-muted", "Ended · 6 seasons · 62 episodes" }
                        }
                    }
                }
            }
        }
    }
}
