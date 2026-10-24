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
        skeleton::{Loaded, Skeleton},
    },
    route::{Route, SettingsPage},
};

/// The settings in effect on `page`, with the pages beside them. A change is stored in the
/// database over the config file and applies at once. `/settings` and an unknown page are
/// replaced by the page's own URL, which its link is marked current against.
#[component]
pub fn Settings(page: SettingsPage) -> Element {
    use_effect(use_reactive!(|page| {
        let canonical = Route::Settings { page };
        if router().full_route_string() != canonical.to_string() {
            navigator().replace(canonical);
        }
    }));
    rsx! {
        document::Title { "{page.title()} · Settings · Yokoku" }
        h1 { class: "yk-page-title", "Settings" }
        p { class: "mt-2 text-muted",
            "Changes apply at once and are kept in the database, over the config file."
        }
        div { class: "mt-8 grid gap-8 md:grid-cols-[10rem_minmax(0,1fr)] md:gap-12",
            nav {
                aria_label: "Settings pages",
                class: "flex overflow-x-auto border-b border-line max-md:pr-8 \
                        max-md:[mask-image:linear-gradient(to_right,#000_calc(100%_-_2rem),transparent)] \
                        md:sticky md:top-24 md:flex-col md:self-start md:border-b-0 md:border-l",
                for each in SettingsPage::ALL {
                    Link {
                        to: Route::Settings { page: each },
                        class: "-mb-px shrink-0 border-b-2 border-transparent px-3 py-2 whitespace-nowrap text-muted \
                                transition-colors hover:text-ink aria-[current=page]:border-ink aria-[current=page]:font-medium \
                                aria-[current=page]:text-ink md:mb-0 md:-ml-px md:border-b-0 md:border-l-2",
                        span {
                            onmounted: move |mounted: MountedEvent| async move {
                                if each == page {
                                    let centered = ScrollToOptions {
                                        behavior: ScrollBehavior::Instant,
                                        vertical: ScrollLogicalPosition::Nearest,
                                        horizontal: ScrollLogicalPosition::Center,
                                    };
                                    _ = mounted.scroll_to_with_options(centered).await;
                                }
                            },
                            "{each.title()}"
                        }
                    }
                }
            }
            div { class: "max-w-3xl",
                Page { key: "{page}", page }
            }
        }
    }
}

/// Loads the settings again for each page, so one saved on another page shows its new value.
#[component]
fn Page(page: SettingsPage) -> Element {
    let loaded = use_server_future(settings)?;
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
            Loaded {
                Groups { settings: all.clone(), page }
            }
        },
    }
}

#[component]
fn Groups(settings: Vec<Setting>, page: SettingsPage) -> Element {
    use_context_provider(|| Unsaved(Signal::new(BTreeMap::new())));
    let of = |section: Section| settings.iter().filter(move |setting| setting.field.section == section).cloned();
    let fields = |section: Section| {
        rsx! {
            for setting in of(section) {
                SettingField { key: "{setting.field.key}", setting }
            }
        }
    };
    let keys = |section: Section| -> Vec<String> { of(section).map(|setting| setting.field.key).collect() };
    rsx! {
        div { class: "grid gap-12",
            match page {
                SettingsPage::Connections => rsx! {
                    Group { title: "Download client",
                        {fields(Section::DownloadClient)}
                        Test {
                            connection: Connection::Transmission,
                            keys: keys(Section::DownloadClient),
                        }
                    }
                    Group { title: "Indexer",
                        {fields(Section::Indexer)}
                        Test { connection: Connection::Jackett, keys: keys(Section::Indexer) }
                    }
                    Group { title: "Media server",
                        {fields(Section::MediaServer)}
                        Test { connection: Connection::Jellyfin, keys: keys(Section::MediaServer) }
                    }
                },
                SettingsPage::Metadata => rsx! {
                    Group { title: "Metadata", {fields(Section::Metadata)} }
                },
                SettingsPage::Library => rsx! {
                    Group { title: "Library",
                        RootFolders {}
                        {fields(Section::Library)}
                    }
                },
                SettingsPage::Import => rsx! {
                    Group { title: "Import",
                        {fields(Section::Import)}
                        {fields(Section::Files)}
                    }
                },
                SettingsPage::Naming => rsx! {
                    Group { title: "Naming",
                        p { class: "text-caption text-muted",
                            "Tokens: "
                            code { class: "yk-code", "{{title}} {{year}} {{season}} {{episodes}} {{episode_title}}" }
                            ". A [...] group is dropped when a token in it has no value. New imports use the patterns; rename existing files from an item's page."
                        }
                        {fields(Section::Naming)}
                    }
                },
                SettingsPage::Notifications => rsx! {
                    Group { title: "ntfy",
                        {fields(Section::Notifications)}
                        Test {
                            connection: Connection::Ntfy,
                            keys: keys(Section::Notifications),
                        }
                    }
                },
                SettingsPage::Schedules => rsx! {
                    Group { title: "Schedules",
                        p { class: "text-caption text-muted",
                            "Cron schedules with seconds: second minute hour day month weekday, like "
                            code { class: "yk-code", "*/30 * * * * *" }
                            ". A change applies from the next run."
                        }
                        {fields(Section::Schedules)}
                    }
                },
                SettingsPage::Server => rsx! {
                    Group { title: "Server",
                        p { class: "text-caption text-muted",
                            "Read when the service starts: change these in the config file or environment, then restart it."
                        }
                        {fields(Section::Server)}
                    }
                },
            }
        }
    }
}

#[component]
fn Group(title: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "grid gap-5 border-t border-line pt-6 max-md:first:border-t-0 max-md:first:pt-0",
            h2 { class: "text-section font-medium", "{title}" }
            {children}
        }
    }
}

/// Checks the connection, or sends a test notification, with the section's values as typed,
/// saved or not.
#[component]
fn Test(connection: Connection, keys: Vec<String>) -> Element {
    let notifies = connection == Connection::Ntfy;
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
                if notifies {
                    "Send test notification"
                } else {
                    "Test connection"
                }
            }
            if outcome().is_none() && !changes().is_empty() {
                span { class: "text-caption text-muted",
                    "Tests the values as typed; save them to keep them."
                }
            }
            span { role: "status", class: "text-caption",
                match outcome() {
                    Some(Ok(topic)) if notifies => rsx! {
                        span { class: "text-success", "Sent a test notification to {topic}" }
                    },
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
