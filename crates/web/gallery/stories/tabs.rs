use dioxus::prelude::*;
use yokoku_web::components::ui::tabs::{TabContent, TabList, TabTrigger, Tabs};

use crate::{Story, StoryPage};

#[component]
pub fn TabsStory() -> Element {
    rsx! {
        StoryPage {
            name: "Tabs",
            path: "ui::tabs",
            summary: "Panels reached by keyboard-navigable buttons, underlined when selected.",
            Story { title: "Views",
                Tabs { default_value: "list".to_string(), horizontal: true,
                    TabList {
                        TabTrigger { value: "list".to_string(), index: 0usize, "List" }
                        TabTrigger { value: "week".to_string(), index: 1usize, "Week" }
                        TabTrigger { value: "month".to_string(), index: 2usize, "Month" }
                    }
                    TabContent { value: "list".to_string(), index: 0usize,
                        p { class: "text-muted", "List panel content." }
                    }
                    TabContent { value: "week".to_string(), index: 1usize,
                        p { class: "text-muted", "Week panel content." }
                    }
                    TabContent { value: "month".to_string(), index: 2usize,
                        p { class: "text-muted", "Month panel content." }
                    }
                }
            }
        }
    }
}
