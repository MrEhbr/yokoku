use dioxus::prelude::*;
use yokoku_web::components::ui::skeleton::Skeleton;

use crate::{Story, StoryPage};

#[component]
pub fn SkeletonStory() -> Element {
    rsx! {
        StoryPage {
            name: "Skeleton",
            path: "ui::skeleton",
            summary: "A placeholder with the shape of content that is loading.",
            Story { title: "Media card",
                div { class: "flex items-center gap-3",
                    Skeleton { class: "h-24 w-16" }
                    div { class: "grid flex-1 gap-2",
                        Skeleton { class: "h-4 w-3/4" }
                        Skeleton { class: "h-4 w-1/2" }
                    }
                }
            }
        }
    }
}
