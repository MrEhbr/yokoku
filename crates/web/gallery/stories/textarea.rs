use dioxus::prelude::*;
use yokoku_web::components::ui::{
    field::{Field, FieldError},
    label::Label,
    textarea::Textarea,
};

use crate::{Story, StoryPage};

#[component]
pub fn TextareaStory() -> Element {
    rsx! {
        StoryPage {
            name: "Textarea",
            path: "ui::textarea",
            summary: "A multi-line text field that grows with its content up to its container.",
            Story { title: "Placeholder",
                div { class: "max-w-md",
                    Field {
                        Label { html_for: "notes", "Notes" }
                        Textarea { id: "notes", placeholder: "Anything to remember" }
                    }
                }
            }
            Story { title: "Invalid",
                div { class: "max-w-md",
                    Field {
                        Label { html_for: "reason", "Skip reason" }
                        Textarea {
                            id: "reason",
                            aria_invalid: "true",
                            aria_describedby: "reason-error",
                        }
                        FieldError { id: "reason-error", "Explain why this file is skipped." }
                    }
                }
            }
        }
    }
}
