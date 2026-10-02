use dioxus::prelude::*;

use crate::{
    api::{
        failure,
        library::Kind,
        settings::{Scanned, add_root, remove_root, roots, scan_library},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        field::{FieldError, FieldHint},
        input::Input,
        label::Label,
        select::{Select, SelectOption},
        skeleton::Skeleton,
    },
    format::plural,
    route::SettingsPart,
};

/// The folders that hold series and movies; one holding items cannot be removed. Scrolls
/// into view once mounted when `scrolled_to`.
#[component]
pub(super) fn RootFolders(scrolled_to: bool) -> Element {
    let mut listed = use_resource(roots);
    let mut error = use_signal(|| None::<String>);
    let mut kind = use_signal(|| Some(Kind::Series));
    let mut path = use_signal(String::new);
    let mut adding = use_signal(|| false);
    let add = move || async move {
        let Some(chosen) = kind() else { return };
        adding.set(true);
        error.set(None);
        match add_root(chosen, path()).await {
            Ok(()) => {
                path.set(String::new());
                listed.restart();
            },
            Err(failed) => error.set(Some(failure(&failed))),
        }
        adding.set(false);
    };
    rsx! {
        div {
            id: SettingsPart::RootFolders.id(),
            class: "grid scroll-mt-28 gap-3",
            onmounted: move |mounted: MountedEvent| async move {
                if scrolled_to {
                    _ = mounted.scroll_to(ScrollBehavior::Smooth).await;
                }
            },
            h3 { class: "font-medium", "Root folders" }
            match &*listed.read() {
                None => rsx! {
                    Skeleton { class: "h-16 w-full" }
                },
                Some(Err(failed)) => rsx! {
                    p { role: "alert", class: "text-danger", {failure(failed)} }
                },
                Some(Ok(folders)) if folders.is_empty() => rsx! {
                    p { class: "text-muted", "No root folders yet. Add one for series and one for movies to add items." }
                },
                Some(Ok(folders)) => rsx! {
                    ul { class: "border-t border-line",
                        for folder in folders.clone() {
                            li {
                                key: "{folder.path}",
                                class: "flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-line py-2",
                                span { class: "w-16 text-caption text-muted",
                                    if folder.kind == Kind::Series {
                                        "Series"
                                    } else {
                                        "Movies"
                                    }
                                }
                                span { class: "yk-code min-w-0 flex-1 [overflow-wrap:anywhere]", "{folder.path}" }
                                span { class: "text-caption text-muted", {plural(folder.items, "item", "items")} }
                                Button {
                                    variant: ButtonVariant::Quiet,
                                    size: ButtonSize::Sm,
                                    disabled: folder.items > 0,
                                    title: if folder.items > 0 { "Items still belong to it" },
                                    aria_label: "Remove {folder.path}",
                                    onclick: move |_| {
                                        let path = folder.path.clone();
                                        async move {
                                            error.set(None);
                                            match remove_root(path).await {
                                                Ok(()) => listed.restart(),
                                                Err(failed) => error.set(Some(failure(&failed))),
                                            }
                                        }
                                    },
                                    "Remove"
                                }
                            }
                        }
                    }
                },
            }
            div { class: "grid gap-1.5",
                Label { html_for: "root-path", "Add a root folder" }
                div { class: "flex flex-wrap gap-2",
                    div { class: "w-32",
                        Select::<Kind> {
                            id: "root-kind",
                            aria_label: "Holds",
                            value: Some(kind.into()),
                            placeholder: "Series",
                            on_value_change: move |next| kind.set(next),
                            SelectOption::<Kind> {
                                index: 0usize,
                                value: Kind::Series,
                                text_value: "Series",
                                "Series"
                            }
                            SelectOption::<Kind> {
                                index: 1usize,
                                value: Kind::Movie,
                                text_value: "Movies",
                                "Movies"
                            }
                        }
                    }
                    div { class: "min-w-48 flex-1",
                        Input {
                            id: "root-path",
                            class: "yk-code",
                            placeholder: "/media/shows",
                            value: "{path}",
                            aria_invalid: error.read().is_some(),
                            aria_describedby: if error.read().is_some() { "root-error" } else { "root-hint" },
                            oninput: move |event: FormEvent| path.set(event.value()),
                            onkeydown: move |event: KeyboardEvent| {
                                if event.key() == Key::Enter && !path.read().trim().is_empty() {
                                    spawn(add());
                                }
                            },
                        }
                    }
                    Button {
                        disabled: adding() || path.read().trim().is_empty(),
                        aria_busy: adding(),
                        onclick: move |_| add(),
                        "Add"
                    }
                }
                if let Some(message) = error() {
                    FieldError { id: "root-error", "{message}" }
                }
                FieldHint { id: "root-hint", "An absolute path to an existing folder on the server." }
            }
            ScanLibrary {}
        }
    }
}

/// Scans the item folders now and says what changed.
#[component]
fn ScanLibrary() -> Element {
    let mut scanning = use_signal(|| false);
    let mut outcome = use_signal(|| None::<Result<Scanned, String>>);
    let scan = move |_| async move {
        scanning.set(true);
        outcome.set(Some(scan_library().await.map_err(|failed| failure(&failed))));
        scanning.set(false);
    };
    rsx! {
        div { class: "flex flex-wrap items-center gap-x-4 gap-y-2 pt-2",
            Button { disabled: scanning(), aria_busy: scanning(), onclick: scan, "Scan now" }
            p { role: "status", class: "text-caption text-muted",
                match outcome() {
                    None if scanning() => "Scanning the item folders…".to_owned(),
                    None => {
                        "The item folders are scanned daily at 05:00 for new and removed files."
                            .to_owned()
                    }
                    Some(Ok(scanned)) => summary(scanned),
                    Some(Err(message)) => message,
                }
            }
        }
    }
}

fn summary(scanned: Scanned) -> String {
    let mut parts = Vec::new();
    if scanned.found > 0 {
        parts.push(format!("Linked {}", plural(scanned.found, "new file", "new files")));
    }
    if scanned.vanished > 0 {
        parts.push(format!("Forgot {} gone from disk", plural(scanned.vanished, "file", "files")));
    }
    if scanned.unrecognised > 0 {
        parts.push(format!(
            "{} with files to match on their item pages",
            plural(scanned.unrecognised, "folder", "folders")
        ));
    }
    if parts.is_empty() {
        return "Nothing changed since the last scan.".to_owned();
    }
    format!("{}.", parts.join(". "))
}
