use dioxus::prelude::*;
use dioxus_icons::lucide::Copy;
use yokoku_web::components::{
    button::{Button, ButtonSize},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};

use crate::{Story, StoryPage};

#[component]
pub fn TooltipStory() -> Element {
    rsx! {
        StoryPage {
            name: "Tooltip",
            path: "tooltip",
            summary: "A short hint shown on hover or focus. The trigger needs its own accessible label too.",
            Story { title: "Icon button (hover it)",
                div { class: "pt-10",
                    Tooltip {
                        TooltipTrigger {
                            r#as: move |attrs: Vec<Attribute>| rsx! {
                                Button { size: ButtonSize::Icon, aria_label: "Copy path", attributes: attrs,
                                    Copy { size: "1rem" }
                                }
                            },
                        }
                        TooltipContent { "Copy path" }
                    }
                }
            }
            Story { title: "Text trigger",
                div { class: "pt-10",
                    Tooltip {
                        TooltipTrigger {
                            span { class: "yk-code underline decoration-dotted", "S01E04–E05" }
                        }
                        TooltipContent { "One file holds two episodes" }
                    }
                }
            }
        }
    }
}
