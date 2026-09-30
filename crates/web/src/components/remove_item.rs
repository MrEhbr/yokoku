use dioxus::prelude::*;
use yokoku_domain::ItemId;

use crate::{
    api::{
        failure,
        library::{detail::DiskUsage, manage::remove},
    },
    components::{
        alert_dialog::{AlertDialog, AlertDialogActions, AlertDialogCancel, AlertDialogDescription, AlertDialogTitle},
        button::{Button, ButtonVariant},
        checkbox::{Checkbox, CheckboxState},
        label::Label,
    },
    format::size,
    route::Route,
};

/// Removes the item from the library after a confirmation that can also delete its files, `usage`
/// (FR-1.7, 8.5); the Library opens once it is removed.
#[component]
pub fn RemoveItem(item: ItemId, title: String, usage: DiskUsage) -> Element {
    let DiskUsage { files, size: bytes } = usage;
    let mut open = use_signal(|| false);
    let mut delete_files = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut failed = use_signal(|| None::<String>);
    let its_files = if files == 1 { "its file".to_owned() } else { format!("its {files} files") };
    let confirm = if delete_files() { format!("Remove and delete {its_files}") } else { "Remove".to_owned() };
    rsx! {
        Button {
            variant: ButtonVariant::Danger,
            onclick: move |_| {
                delete_files.set(false);
                failed.set(None);
                open.set(true);
            },
            "Remove…"
        }
        AlertDialog {
            open: Some(open()),
            on_open_change: move |next| {
                if !busy() {
                    open.set(next);
                }
            },
            AlertDialogTitle { "Remove {title}?" }
            AlertDialogDescription {
                "It leaves the library and is no longer tracked."
                if files == 1 {
                    " Its file stays on disk unless you delete it."
                } else if files > 1 {
                    " Its files stay on disk unless you delete them."
                }
            }
            if files > 0 {
                div { class: "flex flex-col gap-1",
                    div { class: "flex items-center gap-2",
                        Checkbox {
                            id: "remove-delete-files",
                            checked: if delete_files() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                            on_checked_change: move |state| delete_files.set(state == CheckboxState::Checked),
                        }
                        Label { html_for: "remove-delete-files",
                            "Also delete {its_files} ({size(bytes)})"
                        }
                    }
                    if delete_files() {
                        p { class: "pl-6 text-caption text-danger",
                            "Frees up to {size(bytes)}; a file hard-linked to a seeding torrent keeps its space until the torrent is removed. Deleted files are gone for good."
                        }
                    }
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
                        match remove(item, delete_files()).await {
                            Ok(()) => {
                                navigator().replace(Route::Library {});
                            }
                            Err(error) => {
                                failed.set(Some(failure(&error)));
                                busy.set(false);
                            }
                        }
                    },
                    "{confirm}"
                }
            }
        }
    }
}
