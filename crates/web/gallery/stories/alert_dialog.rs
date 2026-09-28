use dioxus::prelude::*;
use yokoku_web::components::ui::{
    alert_dialog::{
        AlertDialog, AlertDialogAction, AlertDialogActions, AlertDialogCancel, AlertDialogDescription, AlertDialogTitle,
    },
    button::Button,
};

use crate::{Story, StoryPage};

#[component]
pub fn AlertDialogStory() -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        StoryPage {
            name: "Alert dialog",
            path: "ui::alert_dialog",
            summary: "A dialog that interrupts to confirm a consequential action.",
            Story { title: "Confirm replacement",
                Button { onclick: move |_| open.set(true), "Replace file…" }
                AlertDialog {
                    open: Some(open()),
                    on_open_change: move |next| open.set(next),
                    AlertDialogTitle { "Replace existing file?" }
                    AlertDialogDescription { "The existing 1.2 GB file is deleted and replaced by the new 1.4 GB file." }
                    AlertDialogActions {
                        AlertDialogCancel { "Keep both" }
                        AlertDialogAction { "Replace file" }
                    }
                }
            }
        }
    }
}
