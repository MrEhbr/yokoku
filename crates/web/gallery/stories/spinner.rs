use dioxus::prelude::*;
use yokoku_web::components::ui::{
    button::{Button, ButtonVariant},
    spinner::Spinner,
};

use crate::{Story, StoryPage};

#[component]
pub fn SpinnerStory() -> Element {
    rsx! {
        StoryPage {
            name: "Spinner",
            path: "ui::spinner",
            summary: "An indeterminate loading indicator with an accessible label, for work with no known progress.",
            Story { title: "Inline",
                div { class: "flex items-center gap-2 text-muted",
                    Spinner {}
                    "Loading"
                }
            }
            Story { title: "In a busy button",
                Button {
                    variant: ButtonVariant::Primary,
                    disabled: true,
                    aria_busy: "true",
                    Spinner { label: "Importing" }
                    "Importing 12 files"
                }
            }
        }
    }
}
