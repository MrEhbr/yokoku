use dioxus::prelude::*;
use yokoku_web::components::ui::accordion::{Accordion, AccordionContent, AccordionItem, AccordionTrigger};

use crate::{Story, StoryPage};

#[component]
pub fn AccordionStory() -> Element {
    rsx! {
        StoryPage {
            name: "Accordion",
            path: "ui::accordion",
            summary: "Disclosure sections. Set `collapsible: false` to keep one item open at all times.",
            Story { title: "Exclusive items",
                Accordion {
                    AccordionItem { index: 0usize,
                        AccordionTrigger { "What counts as missing?" }
                        AccordionContent { "A monitored episode that has aired and has no file." }
                    }
                    AccordionItem { index: 1usize,
                        AccordionTrigger { "How are files renamed?" }
                        AccordionContent { "From saved naming rules and existing matches." }
                    }
                }
            }
        }
    }
}
