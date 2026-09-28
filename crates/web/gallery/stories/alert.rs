use dioxus::prelude::*;
use dioxus_icons::lucide::{Calendar, CircleAlert, Info, TriangleAlert};
use yokoku_web::components::alert::{Alert, AlertDescription, AlertTitle, AlertVariant};

use crate::{Story, StoryPage};

#[component]
pub fn AlertStory() -> Element {
    rsx! {
        StoryPage {
            name: "Alert",
            path: "alert",
            summary: "A notice within the page: a rule and a soft fill in the status color.",
            Story { title: "Neutral",
                Alert {
                    Info {}
                    AlertTitle { "Metadata refresh scheduled" }
                    AlertDescription { "Runs nightly at 03:00." }
                }
            }
            Story { title: "Info, title only",
                Alert { variant: AlertVariant::Info,
                    Calendar {}
                    AlertTitle { "4 episodes air this week" }
                }
            }
            Story { title: "Warning",
                Alert { variant: AlertVariant::Warning,
                    CircleAlert {}
                    AlertTitle { "3 files need review" }
                    AlertDescription { "Detection could not match them to an episode." }
                }
            }
            Story { title: "Danger",
                Alert { variant: AlertVariant::Danger,
                    TriangleAlert {}
                    AlertTitle { "Import failed" }
                    AlertDescription { "The destination disk is full. Free space and retry." }
                }
            }
            Story { title: "Without icon",
                Alert {
                    AlertTitle { "Naming rules apply to new imports only." }
                }
            }
        }
    }
}
