mod fields;
mod roots;

use std::collections::HashMap;

use dioxus::prelude::*;

use self::{
    fields::{Control, SettingField},
    roots::RootFolders,
};
use crate::{
    api::{
        failure,
        settings::{Connection, Setting, settings, test_connection},
    },
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        button::Button,
        skeleton::Skeleton,
    },
};

type Fields = &'static [(&'static str, &'static str, &'static str, Control)];

const DOWNLOADS: Fields = &[
    (
        "transmission.url",
        "Transmission address",
        "Its RPC address.",
        Control::Text("http://localhost:9091/transmission/rpc"),
    ),
    ("transmission.username", "Username", "Empty when Transmission asks for none.", Control::Text("")),
    ("transmission.password", "Password", "", Control::Secret),
];

const MEDIA_SERVER: Fields = &[
    (
        "jellyfin.url",
        "Jellyfin address",
        "Asked to rescan after imports; empty to not use Jellyfin.",
        Control::Text("http://localhost:8096"),
    ),
    ("jellyfin.api_key", "API key", "From Jellyfin's dashboard, under API keys.", Control::Secret),
];

const METADATA: Fields = &[
    (
        "metadata.tmdb.token",
        "TMDB token",
        "The API read access token; needed to search, add and refresh items.",
        Control::Secret,
    ),
    (
        "metadata.tvdb.api_key",
        "TVDB API key",
        "Series come from TVDB once it is set; use the source Jellyfin uses.",
        Control::Secret,
    ),
    ("metadata.tvdb.pin", "TVDB subscriber PIN", "", Control::Secret),
    ("metadata.language", "Language", "Titles and overviews, like en-US.", Control::Text("en-US")),
    ("metadata.region", "Region", "Release dates, like US.", Control::Text("US")),
];

const LIBRARY: Fields = &[
    (
        "add.monitor",
        "Monitor new series",
        "What a new series monitors at first; a new movie is monitored unless this is Nothing.",
        Control::Choice(&[
            ("all", "All episodes"),
            ("future", "Future episodes"),
            ("latest-season", "Latest season"),
            ("none", "Nothing"),
        ]),
    ),
    (
        "clock.timezone",
        "Time zone",
        "For air dates, like Europe/Berlin; the server's own when empty.",
        Control::Text(""),
    ),
];

const IMPORT: Fields = &[
    (
        "import.mode",
        "Import mode",
        "A hard link or copy lets the torrent keep seeding; a move does not.",
        Control::Choice(&[("hardlink", "Hard link"), ("copy", "Copy"), ("move", "Move")]),
    ),
    (
        "downloads.remove_after_seeding",
        "Remove torrents after seeding",
        "Removes imported torrents with their data once Transmission finished seeding them.",
        Control::Switch,
    ),
    (
        "downloads.pick_up_labels",
        "Pick up labels",
        "Takes on torrents added in Transmission with one of these labels, comma-separated.",
        Control::List,
    ),
    ("downloads.pick_up_folder", "Pick up folder", "…or downloading under this folder.", Control::Text("")),
];

const NAMING: Fields = &[
    ("naming.series_folder", "Series folder", "", Control::Text("")),
    ("naming.season_folder", "Season folder", "", Control::Text("")),
    ("naming.episode_file", "Episode file", "", Control::Text("")),
    ("naming.movie_folder", "Movie folder", "", Control::Text("")),
    ("naming.movie_file", "Movie file", "", Control::Text("")),
];

const FILES: Fields = &[(
    "files.ffprobe",
    "ffprobe",
    "Reads file details: its name on the PATH, or a path to it.",
    Control::Text("ffprobe"),
)];

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
                    Sections { settings: all.iter().map(|setting| (setting.key.clone(), setting.clone())).collect::<HashMap<_, _>>() }
                },
            }
        }
    }
}

#[component]
fn Sections(settings: HashMap<String, Setting>) -> Element {
    let group = |fields: Fields| {
        let settings = settings.clone();
        rsx! {
            for (key, label, hint, control) in fields.iter().copied() {
                if let Some(setting) = settings.get(key).cloned() {
                    SettingField { key: "{key}", setting, label, hint, control }
                }
            }
        }
    };
    rsx! {
        div { class: "grid max-w-3xl gap-12",
            Section { title: "Download client", {group(DOWNLOADS)} Test { connection: Connection::Transmission } }
            Section { title: "Media server", {group(MEDIA_SERVER)} Test { connection: Connection::Jellyfin } }
            Section { title: "Metadata", {group(METADATA)} }
            Section { title: "Library",
                RootFolders {}
                {group(LIBRARY)}
            }
            Section { title: "Import", {group(IMPORT)} }
            Section { title: "Naming",
                p { class: "text-caption text-muted",
                    "Tokens: "
                    code { class: "yk-code", "{{title}} {{year}} {{season}} {{episodes}} {{episode_title}}" }
                    ". A [...] group is dropped when a token in it has no value. New imports use the patterns; rename existing files from an item's page."
                }
                {group(NAMING)}
            }
            Section { title: "Files", {group(FILES)} }
        }
    }
}

#[component]
fn Section(title: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "grid gap-5 border-t border-line pt-6",
            h2 { class: "text-section font-medium", "{title}" }
            {children}
        }
    }
}

/// Checks the connection with the settings in effect.
#[component]
fn Test(connection: Connection) -> Element {
    let mut busy = use_signal(|| false);
    let mut outcome = use_signal(|| None::<Result<String, String>>);
    rsx! {
        div { class: "flex flex-wrap items-center gap-3",
            Button {
                disabled: busy(),
                aria_busy: busy(),
                onclick: move |_| async move {
                    busy.set(true);
                    outcome.set(None);
                    outcome.set(Some(test_connection(connection).await.map_err(|error| failure(&error))));
                    busy.set(false);
                },
                "Test connection"
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
