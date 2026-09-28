use dioxus::prelude::*;
use yokoku_web::components::toggle::Toggle;

use crate::{Story, StoryPage};

#[component]
pub fn ToggleStory() -> Element {
    let mut view = use_signal(|| "List");
    rsx! {
        StoryPage {
            name: "Toggle",
            path: "toggle",
            summary: "A button that stays pressed when selected. Give it an `aria_label` if its children are icon-only.",
            Story { title: "Independent",
                div { class: "flex items-center gap-1",
                    Toggle { aria_label: "Bold",
                        em { "B" }
                    }
                    Toggle { aria_label: "Italic", default_pressed: true,
                        em { "I" }
                    }
                }
            }
            Story { title: "Exclusive group",
                div { class: "inline-flex w-fit items-center gap-1 border border-control p-1",
                    for label in ["List", "Week", "Month"] {
                        Toggle {
                            key: "{label}",
                            aria_label: "{label}",
                            pressed: Some(view() == label),
                            on_pressed_change: move |pressed| {
                                if pressed {
                                    view.set(label);
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
