mod fields;
mod roots;

use std::collections::BTreeMap;

use dioxus::prelude::*;
use serde_json::Value;

use self::{
    fields::{SettingField, Unsaved},
    roots::RootFolders,
};
use crate::{
    api::{
        failure,
        settings::{Connection, Section, Setting, settings, test_connection},
    },
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        button::Button,
        skeleton::Skeleton,
    },
};

/// The settings in effect, grouped by what they configure, and the root folders (FR-10.1). A
/// change is stored in the database over the config file and applies at once.
#[component]
pub fn Settings() -> Element {
    let loaded = use_server_future(settings)?;
    rsx! {
        document::Title { "Settings · Yokoku" }
        h1 { class: "yk-page-title", "Settings" }
        p { class: "mt-2 text-muted",
            "Changes apply at once and are kept in the database, over the config file."
        }
        div { class: "mt-8",
            match &*loaded.read() {
                None => rsx! {
                    Skeleton { class: "h-96 w-full" }
                },
                Some(Err(error)) => rsx! {
                    Alert { variant: AlertVariant::Danger,
                        AlertTitle { "The settings could not be loaded" }
                        AlertDescription { {failure(error)} }
                    }
                },
                Some(Ok(all)) => rsx! {
                    Sections { settings: all.clone() }
                },
            }
        }
    }
}

#[component]
fn Sections(settings: Vec<Setting>) -> Element {
    use_context_provider(|| Unsaved(Signal::new(BTreeMap::new())));
    let of = |section: Section| -> Vec<Setting> {
        settings.iter().filter(|setting| setting.field.section == section).cloned().collect()
    };
    let fields = |section: Section| {
        rsx! {
            for setting in of(section) {
                SettingField { key: "{setting.field.key}", setting }
            }
        }
    };
    let keys = |section: Section| -> Vec<String> { of(section).into_iter().map(|setting| setting.field.key).collect() };
    rsx! {
        div { class: "grid max-w-3xl gap-12",
            Group { title: "Download client",
                {fields(Section::DownloadClient)}
                Test {
                    connection: Connection::Transmission,
                    keys: keys(Section::DownloadClient),
                }
            }
            Group { title: "Media server",
                {fields(Section::MediaServer)}
                Test {
                    connection: Connection::Jellyfin,
                    keys: keys(Section::MediaServer),
                }
            }
            Group { title: "Metadata", {fields(Section::Metadata)} }
            Group { title: "Library",
                RootFolders {}
                {fields(Section::Library)}
            }
            Group { title: "Import", {fields(Section::Import)} }
            Group { title: "Naming",
                p { class: "text-caption text-muted",
                    "Tokens: "
                    code { class: "yk-code",
                        "{{title}} {{year}} {{season}} {{episodes}} {{episode_title}}"
                    }
                    ". A [...] group is dropped when a token in it has no value. New imports use the patterns; rename existing files from an item's page."
                }
                {fields(Section::Naming)}
            }
            Group { title: "Files", {fields(Section::Files)} }
        }
    }
}

#[component]
fn Group(title: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "grid gap-5 border-t border-line pt-6",
            h2 { class: "text-section font-medium", "{title}" }
            {children}
        }
    }
}

/// Checks the connection with the section's values as typed, saved or not.
#[component]
fn Test(connection: Connection, keys: Vec<String>) -> Element {
    let mut busy = use_signal(|| false);
    let mut outcome = use_signal(|| None::<Result<String, String>>);
    let Unsaved(unsaved) = use_context();
    let changes = use_memo(move || -> Vec<(String, Value)> {
        unsaved
            .read()
            .iter()
            .filter(|(key, _)| keys.contains(key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    });
    rsx! {
        div { class: "flex flex-wrap items-center gap-3",
            Button {
                disabled: busy(),
                aria_busy: busy(),
                onclick: move |_| async move {
                    busy.set(true);
                    outcome.set(None);
                    outcome
                        .set(
                            Some(
                                test_connection(connection, changes())
                                    .await
                                    .map_err(|error| failure(&error)),
                            ),
                        );
                    busy.set(false);
                },
                "Test connection"
            }
            if outcome().is_none() && !changes().is_empty() {
                span { class: "text-caption text-muted",
                    "Tests the values as typed; save them to keep them."
                }
            }
            span { role: "status", class: "text-caption",
                match outcome() {
                    Some(Ok(version)) => rsx! {
                        span { class: "text-success", "Connected: {version}" }
                    },
                    Some(Err(error)) => rsx! {
                        span { class: "text-danger", "{error}" }
                    },
                    None => rsx! {},
                }
            }
        }
    }
}
