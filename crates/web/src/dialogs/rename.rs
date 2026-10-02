use std::collections::{BTreeMap, HashSet};

use dioxus::prelude::*;
use yokoku_domain::{ItemId, MediaFileId};

use crate::{
    api::{
        failure,
        rename::{RenamePlan, RenameResult, RenameRow, rename_files, rename_preview},
    },
    components::{
        button::{Button, ButtonVariant},
        checkbox::{Checkbox, CheckboxState},
        dialog::{DialogDescription, DialogFooter},
        label::Label,
        skeleton::Skeleton,
    },
    dialogs::ClosableDialog,
};

/// A "Rename files…" button that previews and renames the item's files and calls
/// `on_change` after a rename.
#[component]
pub fn RenameButton(item: ItemId, on_change: Callback) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        Button { onclick: move |_| open.set(true), "Rename files…" }
        ClosableDialog { title: "Rename files", open,
            DialogDescription { "Files move to the names the naming patterns give them, inside the item's folder." }
            if open() {
                Preview {
                    item,
                    on_change,
                    on_close: move |()| open.set(false),
                }
            }
        }
    }
}

/// The plan, read again after each rename.
#[component]
fn Preview(item: ItemId, on_change: Callback, on_close: Callback) -> Element {
    let mut plan = use_resource(move || rename_preview(item));
    let mut result = use_signal(|| None::<RenameResult>);
    let on_renamed = move |renamed: RenameResult| {
        result.set(Some(renamed));
        plan.clear();
        plan.restart();
        on_change(());
    };
    rsx! {
        if let Some(result) = result() {
            Outcome { result }
        }
        match &*plan.read() {
            None => rsx! {
                Skeleton { class: "h-40 w-full" }
                DialogFooter {
                    Button { onclick: move |_| on_close(()), "Cancel" }
                }
            },
            Some(Err(error)) => rsx! {
                p { role: "alert", class: "text-danger", {failure(error)} }
                DialogFooter {
                    Button { onclick: move |_| on_close(()), "Close" }
                }
            },
            Some(Ok(plan)) => rsx! {
                PlanForm {
                    item,
                    plan: plan.clone(),
                    on_renamed,
                    on_close,
                }
            },
        }
    }
}

#[component]
fn Outcome(result: RenameResult) -> Element {
    let noun = if result.renamed == 1 { "file" } else { "files" };
    rsx! {
        div { role: "status", class: "grid gap-1",
            p { class: "text-success", "Renamed {result.renamed} {noun}." }
            for failed in result.failed {
                p { class: "text-caption text-danger [overflow-wrap:anywhere]",
                    "Could not rename {failed.path}: {failed.reason}"
                }
            }
        }
    }
}

/// Every file is selected at first; "Rename N files" renames the selected ones.
#[component]
fn PlanForm(item: ItemId, plan: RenamePlan, on_renamed: Callback<RenameResult>, on_close: Callback) -> Element {
    let all: HashSet<MediaFileId> = plan.renames.iter().map(|row| row.file).collect();
    let mut selected = use_signal({
        let all = all.clone();
        move || all
    });
    let mut renaming = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let count = selected.read().len();
    let header = match count {
        0 => CheckboxState::Unchecked,
        count if count == all.len() => CheckboxState::Checked,
        _ => CheckboxState::Indeterminate,
    };
    let mut seasons: BTreeMap<Option<u16>, Vec<RenameRow>> = BTreeMap::new();
    for row in plan.renames.iter().cloned() {
        seasons.entry(row.season).or_default().push(row);
    }
    let noun = if count == 1 { "file" } else { "files" };
    let submit = move |_| async move {
        renaming.set(true);
        error.set(None);
        let files: Vec<MediaFileId> = selected.read().iter().copied().collect();
        match rename_files(item, files).await {
            Ok(result) => on_renamed(result),
            Err(failed) => {
                error.set(Some(failure(&failed)));
                renaming.set(false);
            },
        }
    };
    rsx! {
        if plan.renames.is_empty() {
            p { class: "text-muted", "Every file already has the name the naming patterns give it." }
        } else {
            div { class: "flex items-center gap-2 border-b border-line pb-2",
                Checkbox {
                    id: "rename-all",
                    checked: header,
                    on_checked_change: move |state| {
                        selected
                            .set(
                                if state == CheckboxState::Checked {
                                    all.clone()
                                } else {
                                    HashSet::new()
                                },
                            );
                    },
                }
                Label { html_for: "rename-all", "{count} of {plan.renames.len()} selected" }
            }
            for (season, rows) in seasons {
                section { key: "{season:?}", class: "grid",
                    if let Some(season) = season {
                        h3 { class: "pt-3 pb-1 text-caption font-medium text-muted",
                            if season == 0 {
                                "Specials"
                            } else {
                                "Season {season}"
                            }
                        }
                    }
                    for row in rows {
                        Row { key: "{row.file}", row, selected }
                    }
                }
            }
        }
        if !plan.skipped.is_empty() {
            details { class: "text-caption",
                summary { class: "cursor-pointer text-muted", "{plan.skipped.len()} left as they are" }
                ul { class: "mt-2 grid gap-1",
                    for skipped in plan.skipped {
                        li {
                            key: "{skipped.path}",
                            class: "[overflow-wrap:anywhere]",
                            span { class: "yk-code", "{skipped.path}" }
                            span { class: "text-muted", ": {skipped.reason}" }
                        }
                    }
                }
            }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            Button { onclick: move |_| on_close(()), "Close" }
            if !plan.renames.is_empty() {
                Button {
                    variant: ButtonVariant::Primary,
                    disabled: renaming() || count == 0,
                    aria_busy: renaming(),
                    onclick: submit,
                    "Rename {count} {noun}"
                }
            }
        }
    }
}

/// A file's current and new path, with its checkbox.
#[component]
fn Row(row: RenameRow, selected: Signal<HashSet<MediaFileId>>) -> Element {
    let id = format!("rename-{}", row.file);
    let file = row.file;
    let checked = selected.read().contains(&file);
    rsx! {
        div { class: "flex items-start gap-3 border-b border-line py-2 last:border-b-0",
            Checkbox {
                id: "{id}",
                class: "mt-0.5",
                checked: if checked { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: move |state| {
                    let mut selected = selected.write();
                    if state == CheckboxState::Checked {
                        selected.insert(file);
                    } else {
                        selected.remove(&file);
                    }
                },
            }
            label {
                r#for: "{id}",
                class: "grid min-w-0 cursor-pointer gap-0.5 text-caption",
                span { class: "yk-code text-muted [overflow-wrap:anywhere]", "{row.from}" }
                span { class: "yk-code [overflow-wrap:anywhere]",
                    span { aria_hidden: "true", "→ " }
                    span { class: "sr-only", "becomes " }
                    "{row.to}"
                }
                if row.subtitles > 0 {
                    span { class: "text-muted",
                        if row.subtitles == 1 {
                            "and its subtitle"
                        } else {
                            "and its {row.subtitles} subtitles"
                        }
                    }
                }
            }
        }
    }
}
