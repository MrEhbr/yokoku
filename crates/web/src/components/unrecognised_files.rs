use dioxus::prelude::*;

use crate::components::{
    alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
    button::{Button, ButtonSize},
};

/// Files a scan found in an item's folder without recognising them (FR-8.3).
#[component]
pub fn UnrecognisedFiles(count: usize) -> Element {
    let title = if count == 1 {
        "1 file in the folder wasn't recognised".to_owned()
    } else {
        format!("{count} files in the folder weren't recognised")
    };
    rsx! {
        Alert { variant: AlertVariant::Warning,
            div { class: "col-start-2 flex flex-wrap items-center justify-between gap-x-4 gap-y-2",
                div {
                    AlertTitle { "{title}" }
                    AlertDescription { "Match them to add them to the library." }
                }
                Button {
                    size: ButtonSize::Sm,
                    disabled: true,
                    title: "Match with `yokoku review` for now",
                    "Match"
                }
            }
        }
    }
}
