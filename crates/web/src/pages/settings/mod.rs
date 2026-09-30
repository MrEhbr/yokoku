mod fields;
mod roots;

use std::collections::{BTreeMap, HashMap};

use dioxus::prelude::*;
use serde_json::Value;

use self::{
    fields::{Control, SettingField, Unsaved},
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
    ("metadata.language", "Language", "Titles and overviews.", Control::Choice(LANGUAGES)),
    ("metadata.region", "Region", "Whose movie release dates are shown.", Control::Choice(REGIONS)),
];

/// TMDB's translations.
const LANGUAGES: &[(&str, &str)] = &[
    ("ar-SA", "Arabic · ar-SA"),
    ("bg-BG", "Bulgarian · bg-BG"),
    ("zh-CN", "Chinese, Simplified · zh-CN"),
    ("zh-TW", "Chinese, Traditional · zh-TW"),
    ("zh-HK", "Chinese, Hong Kong · zh-HK"),
    ("hr-HR", "Croatian · hr-HR"),
    ("cs-CZ", "Czech · cs-CZ"),
    ("da-DK", "Danish · da-DK"),
    ("nl-NL", "Dutch · nl-NL"),
    ("en-US", "English, US · en-US"),
    ("en-GB", "English, UK · en-GB"),
    ("en-AU", "English, Australia · en-AU"),
    ("en-CA", "English, Canada · en-CA"),
    ("fi-FI", "Finnish · fi-FI"),
    ("fr-FR", "French · fr-FR"),
    ("fr-CA", "French, Canada · fr-CA"),
    ("de-DE", "German · de-DE"),
    ("el-GR", "Greek · el-GR"),
    ("he-IL", "Hebrew · he-IL"),
    ("hi-IN", "Hindi · hi-IN"),
    ("hu-HU", "Hungarian · hu-HU"),
    ("id-ID", "Indonesian · id-ID"),
    ("it-IT", "Italian · it-IT"),
    ("ja-JP", "Japanese · ja-JP"),
    ("ko-KR", "Korean · ko-KR"),
    ("lt-LT", "Lithuanian · lt-LT"),
    ("lv-LV", "Latvian · lv-LV"),
    ("nb-NO", "Norwegian · nb-NO"),
    ("fa-IR", "Persian · fa-IR"),
    ("pl-PL", "Polish · pl-PL"),
    ("pt-BR", "Portuguese, Brazil · pt-BR"),
    ("pt-PT", "Portuguese, Portugal · pt-PT"),
    ("ro-RO", "Romanian · ro-RO"),
    ("ru-RU", "Russian · ru-RU"),
    ("sr-RS", "Serbian · sr-RS"),
    ("sk-SK", "Slovak · sk-SK"),
    ("sl-SI", "Slovenian · sl-SI"),
    ("es-ES", "Spanish, Spain · es-ES"),
    ("es-MX", "Spanish, Latin America · es-MX"),
    ("sv-SE", "Swedish · sv-SE"),
    ("th-TH", "Thai · th-TH"),
    ("tr-TR", "Turkish · tr-TR"),
    ("uk-UA", "Ukrainian · uk-UA"),
    ("vi-VN", "Vietnamese · vi-VN"),
];

/// Countries, by ISO 3166-1 code.
const REGIONS: &[(&str, &str)] = &[
    ("AR", "Argentina · AR"),
    ("AU", "Australia · AU"),
    ("AT", "Austria · AT"),
    ("BE", "Belgium · BE"),
    ("BR", "Brazil · BR"),
    ("BG", "Bulgaria · BG"),
    ("CA", "Canada · CA"),
    ("CL", "Chile · CL"),
    ("CN", "China · CN"),
    ("CO", "Colombia · CO"),
    ("HR", "Croatia · HR"),
    ("CZ", "Czechia · CZ"),
    ("DK", "Denmark · DK"),
    ("EG", "Egypt · EG"),
    ("EE", "Estonia · EE"),
    ("FI", "Finland · FI"),
    ("FR", "France · FR"),
    ("DE", "Germany · DE"),
    ("GR", "Greece · GR"),
    ("HK", "Hong Kong · HK"),
    ("HU", "Hungary · HU"),
    ("IS", "Iceland · IS"),
    ("IN", "India · IN"),
    ("ID", "Indonesia · ID"),
    ("IE", "Ireland · IE"),
    ("IL", "Israel · IL"),
    ("IT", "Italy · IT"),
    ("JP", "Japan · JP"),
    ("KZ", "Kazakhstan · KZ"),
    ("LV", "Latvia · LV"),
    ("LT", "Lithuania · LT"),
    ("MY", "Malaysia · MY"),
    ("MX", "Mexico · MX"),
    ("NL", "Netherlands · NL"),
    ("NZ", "New Zealand · NZ"),
    ("NG", "Nigeria · NG"),
    ("NO", "Norway · NO"),
    ("PE", "Peru · PE"),
    ("PH", "Philippines · PH"),
    ("PL", "Poland · PL"),
    ("PT", "Portugal · PT"),
    ("RO", "Romania · RO"),
    ("RU", "Russia · RU"),
    ("SA", "Saudi Arabia · SA"),
    ("RS", "Serbia · RS"),
    ("SG", "Singapore · SG"),
    ("SK", "Slovakia · SK"),
    ("SI", "Slovenia · SI"),
    ("ZA", "South Africa · ZA"),
    ("KR", "South Korea · KR"),
    ("ES", "Spain · ES"),
    ("SE", "Sweden · SE"),
    ("CH", "Switzerland · CH"),
    ("TW", "Taiwan · TW"),
    ("TH", "Thailand · TH"),
    ("TR", "Türkiye · TR"),
    ("UA", "Ukraine · UA"),
    ("AE", "United Arab Emirates · AE"),
    ("GB", "United Kingdom · GB"),
    ("US", "United States · US"),
    ("VN", "Vietnam · VN"),
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
    use_context_provider(|| Unsaved(Signal::new(BTreeMap::new())));
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

/// Checks the connection with the section's values as typed, saved or not.
#[component]
fn Test(connection: Connection) -> Element {
    let mut busy = use_signal(|| false);
    let mut outcome = use_signal(|| None::<Result<String, String>>);
    let Unsaved(unsaved) = use_context();
    let prefix = match connection {
        Connection::Transmission => "transmission.",
        Connection::Jellyfin => "jellyfin.",
    };
    let changes = move || -> Vec<(String, Value)> {
        unsaved
            .read()
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    };
    rsx! {
        div { class: "flex flex-wrap items-center gap-3",
            Button {
                disabled: busy(),
                aria_busy: busy(),
                onclick: move |_| async move {
                    busy.set(true);
                    outcome.set(None);
                    outcome.set(Some(test_connection(connection, changes()).await.map_err(|error| failure(&error))));
                    busy.set(false);
                },
                "Test connection"
            }
            if outcome().is_none() && !changes().is_empty() {
                span { class: "text-caption text-muted", "Tests the values as typed; save them to keep them." }
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
