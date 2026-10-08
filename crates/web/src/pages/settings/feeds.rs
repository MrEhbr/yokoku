use dioxus::prelude::*;

use crate::{
    api::{
        failure,
        feeds::{FeedDraft, FeedEntry, feeds, remove_feed, save_feed, test_feed},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        input::Input,
        skeleton::Skeleton,
    },
};

/// Direct Torznab feeds. File configured feeds can be tested here and changed in TOML.
#[component]
pub(super) fn Feeds() -> Element {
    let mut listed = use_resource(feeds);
    let mut draft = use_signal(|| None::<FeedDraft>);
    let mut message = use_signal(|| None::<Result<String, String>>);
    let mut busy = use_signal(|| false);
    let mut removing = use_signal(|| None::<String>);
    let saved_key_set = if let (Some(current), Some(Ok(entries))) = (draft(), &*listed.read()) {
        entries.iter().any(|entry| Some(&entry.id) == current.id.as_ref() && entry.key_set)
    } else {
        false
    };
    let new_feed = || FeedDraft { id: None, name: String::new(), url: String::new(), key: None, enabled: true };
    let save = move |_| async move {
        let Some(value) = draft() else { return };
        busy.set(true);
        message.set(None);
        match save_feed(value).await {
            Ok(()) => {
                draft.set(None);
                listed.restart();
            },
            Err(error) => message.set(Some(Err(failure(&error)))),
        }
        busy.set(false);
    };
    let test = move |_| async move {
        let Some(value) = draft() else { return };
        busy.set(true);
        message.set(Some(
            test_feed(value).await.map(|name| format!("Connected: {name}")).map_err(|error| failure(&error)),
        ));
        busy.set(false);
    };
    rsx! {
        div { class: "grid gap-3 border-t border-line pt-5",
            h3 { class: "font-medium", "Direct Torznab feeds" }
            p { class: "text-caption text-muted", "Add the full Torznab API URL supplied by your indexer or indexer manager. Its API key stays on the server." }
            match &*listed.read() {
                None => rsx! { Skeleton { class: "h-24 w-full" } },
                Some(Err(error)) => rsx! { p { role: "alert", class: "text-danger", {failure(error)} } },
                Some(Ok(entries)) if entries.is_empty() => rsx! { p { class: "text-muted", "No direct feeds configured." } },
                Some(Ok(entries)) => rsx! {
                    ul { class: "divide-y divide-line border-y border-line",
                        for entry in entries.clone() {
                            li { key: "{entry.id}", class: "flex flex-wrap items-center gap-3 py-3",
                                div { class: "grid min-w-0 flex-1",
                                    span { class: "font-medium", "{entry.name}" }
                                    span { class: "yk-code truncate text-caption text-muted", title: "{entry.url}", "{entry.url}" }
                                    span { class: "text-caption text-muted",
                                        if !entry.enabled { "Disabled · " }
                                        if entry.configured { "Config file" } else if entry.key_set { "API key set" }
                                    }
                                }
                                Button { size: ButtonSize::Sm, variant: ButtonVariant::Quiet,
                                    disabled: busy(),
                                    onclick: {
                                        let entry = entry.clone();
                                        move |_| { let entry = entry.clone(); async move {
                                            busy.set(true);
                                            message.set(Some(test_feed(as_draft(&entry)).await.map(|name| format!("Connected: {name}")).map_err(|error| failure(&error))));
                                            busy.set(false);
                                        } }
                                    }, "Test" }
                                if !entry.configured {
                                    Button { size: ButtonSize::Sm, variant: ButtonVariant::Quiet,
                                        disabled: busy(),
                                        onclick: {
                                            let entry = entry.clone();
                                            move |_| { draft.set(Some(as_draft(&entry))); message.set(None); }
                                        }, "Edit" }
                                    if removing() == Some(entry.id.clone()) {
                                        Button { size: ButtonSize::Sm, variant: ButtonVariant::Quiet,
                                            disabled: busy(),
                                            onclick: { let id = entry.id.clone(); move |_| { let id = id.clone(); async move {
                                                busy.set(true);
                                                match remove_feed(id).await {
                                                    Ok(()) => { removing.set(None); listed.restart(); },
                                                    Err(error) => message.set(Some(Err(failure(&error)))),
                                                }
                                                busy.set(false);
                                            } } }, "Confirm remove" }
                                    } else {
                                        Button { size: ButtonSize::Sm, variant: ButtonVariant::Quiet,
                                            disabled: busy(),
                                            onclick: { let id = entry.id.clone(); move |_| removing.set(Some(id.clone())) }, "Remove" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
            if draft.read().is_none() {
                Button { class: "w-fit", disabled: busy(), onclick: move |_| { draft.set(Some(new_feed())); message.set(None); }, "Add feed" }
            }
            if let Some(current) = draft() {
                div { class: "grid gap-3 rounded border border-line p-4",
                    h4 { class: "font-medium", if current.id.is_some() { "Edit feed" } else { "Add feed" } }
                    label { class: "grid gap-1", "Name"
                        Input { value: "{current.name}", oninput: move |event: FormEvent| draft.with_mut(|draft| if let Some(feed) = draft { feed.name = event.value(); }) }
                    }
                    label { class: "grid gap-1", "Torznab API URL"
                        Input { value: "{current.url}", placeholder: "https://indexer.example/api",
                            oninput: move |event: FormEvent| draft.with_mut(|draft| if let Some(feed) = draft { feed.url = event.value(); }) }
                    }
                    label { class: "grid gap-1", "API key (optional)"
                        Input { r#type: "password", autocomplete: "off", value: "{current.key.clone().unwrap_or_default()}",
                            placeholder: if saved_key_set { "Leave blank to keep the current key" } else { "API key" },
                            oninput: move |event: FormEvent| draft.with_mut(|draft| if let Some(feed) = draft {
                                let value = event.value();
                                feed.key = (!value.is_empty()).then_some(value);
                            }) }
                    }
                    if saved_key_set {
                        Button { size: ButtonSize::Sm, variant: ButtonVariant::Quiet, class: "w-fit",
                            onclick: move |_| draft.with_mut(|draft| if let Some(feed) = draft {
                                feed.key = if feed.key.as_deref() == Some("") { None } else { Some(String::new()) };
                            }),
                            if current.key.as_deref() == Some("") { "Keep saved API key" } else { "Remove saved API key" }
                        }
                        if current.key.as_deref() == Some("") {
                            p { class: "text-caption text-muted", "The saved API key will be removed when you save." }
                        }
                    }
                    label { class: "flex items-center gap-2",
                        input { r#type: "checkbox", checked: current.enabled,
                            onchange: move |event: FormEvent| draft.with_mut(|draft| if let Some(feed) = draft { feed.enabled = event.checked(); }) }
                        "Enabled"
                    }
                    div { class: "flex gap-2",
                        Button { disabled: busy() || current.name.trim().is_empty() || current.url.trim().is_empty(), onclick: save, "Save" }
                        Button { disabled: busy() || current.url.trim().is_empty(), onclick: test, "Test" }
                        Button { disabled: busy(), onclick: move |_| { draft.set(None); message.set(None); }, "Cancel" }
                    }
                }
            }
            if let Some(result) = message() {
                p { role: "status", class: if result.is_ok() { "text-muted" } else { "text-danger" }, "{result.as_ref().unwrap_or_else(|message| message)}" }
            }
        }
    }
}

fn as_draft(entry: &FeedEntry) -> FeedDraft {
    FeedDraft {
        id: Some(entry.id.clone()),
        name: entry.name.clone(),
        url: entry.url.clone(),
        key: None,
        enabled: entry.enabled,
    }
}
