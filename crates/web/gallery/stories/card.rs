use dioxus::prelude::*;
use yokoku_web::components::ui::{
    button::{Button, ButtonSize, ButtonVariant},
    card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle},
};

use crate::{Story, StoryPage};

#[component]
pub fn CardStory() -> Element {
    rsx! {
        StoryPage {
            name: "Card",
            path: "ui::card",
            summary: "A bordered panel with header, content, and footer.",
            Story { title: "Connection",
                div { class: "max-w-sm",
                    Card {
                        CardHeader {
                            CardTitle { "Jellyfin" }
                            CardDescription { "Connected · 10.10.3" }
                        }
                        CardContent {
                            p { "Libraries refresh after every import." }
                        }
                        CardFooter {
                            Button { size: ButtonSize::Sm, "Test connection" }
                            Button {
                                variant: ButtonVariant::Quiet,
                                size: ButtonSize::Sm,
                                "Disconnect"
                            }
                        }
                    }
                }
            }
        }
    }
}
