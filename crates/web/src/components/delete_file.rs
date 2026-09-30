use dioxus::prelude::*;

use crate::{
    api::{
        failure,
        library::manage::{FileOf, delete_file},
    },
    components::{
        alert_dialog::{AlertDialog, AlertDialogActions, AlertDialogCancel, AlertDialogDescription, AlertDialogTitle},
        button::{Button, ButtonSize, ButtonVariant},
    },
};

/// Deletes the file at `path` from disk after a confirmation (FR-8.4, 8.5), then calls
/// `on_change`. `also` names the other episodes the file holds, which lose it too.
#[component]
pub fn DeleteFile(target: FileOf, path: String, #[props(default)] also: Vec<String>, on_change: Callback) -> Element {
    let mut open = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut failed = use_signal(|| None::<String>);
    rsx! {
        Button {
            variant: ButtonVariant::Danger,
            size: ButtonSize::Sm,
            onclick: move |_| {
                failed.set(None);
                open.set(true);
            },
            "Delete file…"
        }
        AlertDialog {
            open: Some(open()),
            on_open_change: move |next| {
                if !busy() {
                    open.set(next);
                }
            },
            AlertDialogTitle { "Delete this file?" }
            AlertDialogDescription { "It is removed from disk for good, with its subtitles." }
            p { class: "yk-code break-all text-caption", "{path}" }
            if !also.is_empty() {
                p { class: "text-caption text-warning",
                    "It also holds {also.join(\", \")}, which lose it too."
                }
            }
            if let Some(error) = failed() {
                p { role: "alert", class: "text-caption text-danger", "{error}" }
            }
            AlertDialogActions {
                AlertDialogCancel { "Cancel" }
                Button {
                    variant: ButtonVariant::Danger,
                    disabled: busy(),
                    aria_busy: busy(),
                    onclick: move |_| async move {
                        busy.set(true);
                        failed.set(None);
                        let deleted = delete_file(target).await;
                        busy.set(false);
                        match deleted {
                            Ok(()) => {
                                open.set(false);
                                on_change(());
                            }
                            Err(error) => failed.set(Some(failure(&error))),
                        }
                    },
                    "Delete file"
                }
            }
        }
    }
}
