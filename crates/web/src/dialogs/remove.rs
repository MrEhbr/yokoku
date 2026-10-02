use std::collections::{BTreeMap, HashSet};

use dioxus::prelude::*;
use yokoku_domain::{ItemId, MediaFileId};

use crate::{
    api::{
        failure,
        library::manage::{ItemFile, delete_files, item_files, remove},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        checkbox::{Checkbox, CheckboxState},
        dialog::{DialogDescription, DialogFooter},
        item_status::FileWatched,
        label::Label,
        skeleton::Skeleton,
    },
    dialogs::ClosableDialog,
    format::size,
    route::Route,
};

/// A "Remove…" button: deletes the chosen files of the item, or removes it from the library after
/// deleting the chosen ones, its other files staying on disk. `on_change` runs
/// after files of an item that stays are deleted; the Library opens once the item is removed.
#[component]
pub fn RemoveButton(item: ItemId, title: String, on_change: Callback) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        Button { variant: ButtonVariant::Danger, onclick: move |_| open.set(true), "Remove…" }
        ClosableDialog { title: "Remove or delete files", open, wide: true,
            if open() {
                Files {
                    item,
                    title,
                    on_change,
                    on_close: move |()| open.set(false),
                }
            }
        }
    }
}

#[component]
fn Files(item: ItemId, title: String, on_change: Callback, on_close: Callback) -> Element {
    let files = use_resource(move || item_files(item));
    rsx! {
        match &*files.read() {
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
            Some(Ok(files)) => rsx! {
                Form {
                    item,
                    title: title.clone(),
                    files: files.clone(),
                    on_change,
                    on_close,
                }
            },
        }
    }
}

/// No file is chosen at first.
#[component]
fn Form(item: ItemId, title: String, files: Vec<ItemFile>, on_change: Callback, on_close: Callback) -> Element {
    let mut chosen = use_signal(HashSet::<MediaFileId>::new);
    let mut unmonitor = use_signal(|| true);
    let mut remove_item = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let all: HashSet<MediaFileId> = files.iter().map(|file| file.id).collect();
    let watched: HashSet<MediaFileId> =
        files.iter().filter(|file| file.watched.is_some()).map(|file| file.id).collect();
    let count = chosen.read().len();
    let bytes: u64 = files.iter().filter(|file| chosen.read().contains(&file.id)).map(|file| file.size).sum();
    let mut seasons: BTreeMap<Option<u16>, Vec<ItemFile>> = BTreeMap::new();
    for file in files.iter().cloned() {
        seasons.entry(file.season).or_default().push(file);
    }
    let noun = if count == 1 { "file" } else { "files" };
    let confirm = match (remove_item(), count) {
        (true, 0) => "Remove".to_owned(),
        (true, count) => format!("Remove and delete {count} {noun}"),
        (false, count) => format!("Delete {count} {noun}"),
    };
    let submit = move |_| async move {
        busy.set(true);
        error.set(None);
        let files: Vec<MediaFileId> = chosen.read().iter().copied().collect();
        let done = if remove_item() {
            remove(item, files).await.map(|()| {
                navigator().replace(Route::Library {});
            })
        } else {
            delete_files(item, files, unmonitor()).await.map(|()| {
                on_change(());
                on_close(());
            })
        };
        if let Err(failed) = done {
            error.set(Some(failure(&failed)));
            busy.set(false);
        }
    };
    rsx! {
        DialogDescription {
            "Deleted files are gone for good. A file hard-linked to a seeding torrent keeps its space until the torrent is removed."
        }
        if files.is_empty() {
            p { class: "text-muted", "It has no files in the library." }
        } else {
            div { class: "flex flex-wrap items-center gap-2",
                span { class: "text-caption text-muted", "Choose" }
                Button {
                    size: ButtonSize::Sm,
                    onclick: move |_| chosen.set(all.clone()),
                    "All"
                }
                Button {
                    size: ButtonSize::Sm,
                    disabled: watched.is_empty(),
                    onclick: move |_| chosen.set(watched.clone()),
                    "Watched"
                }
                Button { size: ButtonSize::Sm, onclick: move |_| chosen.set(HashSet::new()), "None" }
            }
            div { class: "max-h-[50dvh] overflow-y-auto border-y border-line",
                for (season, files) in seasons {
                    Season { key: "{season:?}", season, files, chosen }
                }
            }
        }
        div { class: "grid gap-2",
            if !remove_item() && !files.is_empty() {
                div { class: "flex items-center gap-2",
                    Checkbox {
                        id: "remove-unmonitor",
                        checked: if unmonitor() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                        on_checked_change: move |state| unmonitor.set(state == CheckboxState::Checked),
                    }
                    Label { html_for: "remove-unmonitor", "Stop monitoring what's deleted" }
                }
            }
            div { class: "flex items-center gap-2",
                Checkbox {
                    id: "remove-item",
                    checked: if remove_item() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: move |state| remove_item.set(state == CheckboxState::Checked),
                }
                Label { html_for: "remove-item", "Also remove {title} from the library" }
            }
            if remove_item() && !files.is_empty() {
                p { class: "pl-6 text-caption text-muted",
                    "It is no longer tracked; files you don't choose stay on disk."
                }
            }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            if count > 0 {
                span { class: "mr-auto text-caption text-muted", "{size(bytes)} chosen" }
            }
            Button { onclick: move |_| on_close(()), "Cancel" }
            Button {
                variant: ButtonVariant::Danger,
                disabled: busy() || (!remove_item() && count == 0),
                aria_busy: busy(),
                onclick: submit,
                "{confirm}"
            }
        }
    }
}

/// A season's files with a checkbox choosing them all; a movie's file without one.
#[component]
fn Season(season: Option<u16>, files: Vec<ItemFile>, chosen: Signal<HashSet<MediaFileId>>) -> Element {
    let ids: Vec<MediaFileId> = files.iter().map(|file| file.id).collect();
    let picked = ids.iter().filter(|id| chosen.read().contains(id)).count();
    let state = match picked {
        0 => CheckboxState::Unchecked,
        picked if picked == ids.len() => CheckboxState::Checked,
        _ => CheckboxState::Indeterminate,
    };
    let name = match season {
        Some(0) => "Specials".to_owned(),
        Some(number) => format!("Season {number}"),
        None => String::new(),
    };
    rsx! {
        section { class: "grid",
            if let Some(number) = season {
                div { class: "sticky top-0 z-10 flex items-center gap-2 border-b border-line bg-surface py-2",
                    Checkbox {
                        id: "remove-season-{number}",
                        checked: state,
                        on_checked_change: move |state| {
                            let mut chosen = chosen.write();
                            for id in &ids {
                                if state == CheckboxState::Checked {
                                    chosen.insert(*id);
                                } else {
                                    chosen.remove(id);
                                }
                            }
                        },
                    }
                    Label { html_for: "remove-season-{number}", class: "font-medium",
                        "{name}"
                        span { class: "ml-2 font-normal text-caption text-muted", "{picked} of {files.len()}" }
                    }
                }
            }
            for file in files {
                Row { key: "{file.id}", file, chosen }
            }
        }
    }
}

#[component]
fn Row(file: ItemFile, chosen: Signal<HashSet<MediaFileId>>) -> Element {
    let id = format!("remove-{}", file.id);
    let file_id = file.id;
    let checked = chosen.read().contains(&file_id);
    rsx! {
        div { class: "flex items-center gap-3 border-b border-line py-2 pl-6 last:border-b-0",
            Checkbox {
                id: "{id}",
                checked: if checked { CheckboxState::Checked } else { CheckboxState::Unchecked },
                on_checked_change: move |state| {
                    let mut chosen = chosen.write();
                    if state == CheckboxState::Checked {
                        chosen.insert(file_id);
                    } else {
                        chosen.remove(&file_id);
                    }
                },
            }
            label {
                r#for: "{id}",
                class: "flex min-w-0 flex-1 cursor-pointer items-baseline gap-3",
                if !file.episodes.is_empty() {
                    span { class: "yk-code shrink-0 whitespace-nowrap", "{file.episodes}" }
                }
                span { class: "min-w-0 truncate", "{file.title}" }
            }
            FileWatched { watched: file.watched }
            span { class: "w-20 shrink-0 text-right text-caption text-muted tabular-nums", "{size(file.size)}" }
        }
    }
}
