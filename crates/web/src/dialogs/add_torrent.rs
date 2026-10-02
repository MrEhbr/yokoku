use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use yokoku_domain::{ItemId, ItemName, SeriesId};

use crate::{
    api::{
        downloads::{ItemLink, NewTorrent, add_torrent},
        failure,
        library::{Entry, library},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        combobox::{Combobox, ComboboxEmpty, ComboboxOption},
        dialog::DialogFooter,
        field::{Field, FieldError, FieldHint},
        input::Input,
        label::Label,
        skeleton::Skeleton,
    },
    dialogs::{ClosableDialog, pickers::SeasonPicker},
    route::Route,
};

/// Bytes; .torrent files are kilobytes.
const MAX_TORRENT_FILE: u64 = 10_000_000;

/// A picked .torrent file.
#[derive(Clone, PartialEq)]
struct TorrentFile {
    name: String,
    bytes: Vec<u8>,
}

/// An "Add torrent" button that opens [`AddTorrent`] for `item`.
#[component]
pub fn AddTorrentButton(#[props(default)] item: Option<ItemLink>) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        Button { variant: ButtonVariant::Primary, onclick: move |_| open.set(true), "Add torrent" }
        AddTorrent { open, item }
    }
}

/// Adds a magnet link or .torrent file for `item`, or without one for a library item chosen in
/// the dialog or for detection to work out; the Downloads page opens once it is added.
/// `open` closes it.
#[component]
fn AddTorrent(open: Signal<bool>, item: Option<ItemLink>) -> Element {
    rsx! {
        ClosableDialog { title: "Add torrent", open,
            if open() {
                Form { item, on_close: move |()| open.set(false) }
            }
        }
    }
}

#[component]
fn Form(item: Option<ItemLink>, on_close: Callback) -> Element {
    let mut magnet = use_signal(String::new);
    let mut file = use_signal(|| None::<TorrentFile>);
    let chosen = use_signal(|| Some(item.as_ref().map(|item| item.id)));
    let season = use_signal(|| None::<u16>);
    let series = match chosen().flatten() {
        Some(ItemId::Series(id)) => Some(id),
        _ => None,
    };
    let mut adding = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let typed = magnet.read().trim().to_owned();
    let not_a_magnet = !typed.is_empty() && !typed.starts_with("magnet:");
    let ready = file.read().is_some() || !(typed.is_empty() || not_a_magnet);
    let submit = move |_| async move {
        let torrent = match file() {
            Some(file) => NewTorrent::File(file.bytes),
            None => NewTorrent::Magnet(magnet()),
        };
        adding.set(true);
        error.set(None);
        match add_torrent(torrent, chosen().flatten(), season().filter(|_| series.is_some())).await {
            Ok(()) => {
                on_close(());
                navigator().push(Route::Downloads {});
            },
            Err(failed) => {
                error.set(Some(failure(&failed)));
                adding.set(false);
            },
        }
    };
    rsx! {
        Field {
            Label { html_for: "torrent-magnet", "Magnet link" }
            Input {
                id: "torrent-magnet",
                placeholder: "magnet:?xt=urn:btih:…",
                value: "{magnet}",
                disabled: file.read().is_some(),
                aria_invalid: not_a_magnet,
                aria_describedby: if not_a_magnet { "torrent-magnet-error" },
                oninput: move |event: FormEvent| magnet.set(event.value()),
            }
            if not_a_magnet {
                FieldError { id: "torrent-magnet-error", "Paste a link that starts with magnet:" }
            }
        }
        Field {
            span { class: "text-caption text-muted", "or" }
            match file() {
                Some(picked) => rsx! {
                    div { class: "flex min-w-0 items-center gap-2",
                        span { class: "yk-code min-w-0 truncate", "{picked.name}" }
                        Button {
                            variant: ButtonVariant::Quiet,
                            size: ButtonSize::Icon,
                            aria_label: "Remove {picked.name}",
                            onclick: move |_| file.set(None),
                            X {}
                        }
                    }
                },
                None => rsx! {
                    label { class: "yk-button w-fit cursor-pointer focus-within:outline-2 focus-within:outline-offset-3 focus-within:outline-ink",
                        "Choose a .torrent file…"
                        input {
                            class: "sr-only",
                            r#type: "file",
                            accept: ".torrent,application/x-bittorrent",
                            onchange: move |event: FormEvent| async move {
                                let Some(picked) = event.files().into_iter().next() else { return };
                                if picked.size() > MAX_TORRENT_FILE {
                                    error
                                        .set(
                                            Some(format!("{} is too large for a .torrent file", picked.name())),
                                        );
                                    return;
                                }
                                match picked.read_bytes().await {
                                    Ok(bytes) => {
                                        file.set(
                                            Some(TorrentFile {
                                                name: picked.name(),
                                                bytes: bytes.to_vec(),
                                            }),
                                        )
                                    }
                                    Err(_) => error.set(Some(format!("{} could not be read", picked.name()))),
                                }
                            },
                        }
                    }
                },
            }
        }
        match &item {
            Some(item) => rsx! {
                p { class: "text-body text-muted", "Its files are imported into {item.title}." }
            },
            None => rsx! {
                ItemField { chosen }
            },
        }
        if let Some(series) = series {
            SeasonField { key: "{series}", series, season }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            Button { onclick: move |_| on_close(()), "Cancel" }
            Button {
                variant: ButtonVariant::Primary,
                disabled: adding() || !ready,
                aria_busy: adding(),
                onclick: submit,
                "Add torrent"
            }
        }
    }
}

/// The season of the series' files whose names give none, or none to leave it to their names.
#[component]
fn SeasonField(series: SeriesId, season: Signal<Option<u16>>) -> Element {
    rsx! {
        Field {
            SeasonPicker {
                id: "torrent-season",
                series,
                season,
                none: Some("From the file names"),
                aria_describedby: Some("torrent-season-hint"),
            }
            FieldHint { id: "torrent-season-hint",
                "For files named without a season, like “Frieren - 05.mkv”; names with one keep it."
            }
        }
    }
}

/// A choice of library item, or none for detection.
#[component]
fn ItemField(chosen: Signal<Option<Option<ItemId>>>) -> Element {
    let items = use_resource(|| library(None, None, None, None));
    rsx! {
        Field {
            Label { html_for: "torrent-item", "For" }
            match &*items.read() {
                None => rsx! {
                    Skeleton { class: "h-9 w-full" }
                },
                Some(Err(failed)) => rsx! {
                    p { role: "alert", class: "text-caption text-danger", {failure(failed)} }
                },
                Some(Ok(entries)) => rsx! {
                    ItemSelect { entries: entries.clone(), chosen }
                },
            }
            FieldHint { id: "torrent-item-hint",
                if chosen().flatten().is_some() {
                    "Its files are imported into this item."
                } else {
                    "Its files are matched to library items by their names."
                }
            }
        }
    }
}

/// The library items, or none for detection.
#[component]
fn ItemSelect(entries: Vec<Entry>, chosen: Signal<Option<Option<ItemId>>>) -> Element {
    let choices: Vec<(Option<ItemId>, String)> = std::iter::once((None, "Work it out from its files".to_owned()))
        .chain(entries.iter().map(|entry| {
            (Some(entry.id), format!("{} · {}", ItemName::new(&entry.title, entry.year), entry.id.kind()))
        }))
        .collect();
    rsx! {
        Combobox::<Option<ItemId>> {
            id: "torrent-item",
            aria_describedby: "torrent-item-hint",
            value: Some(chosen.into()),
            placeholder: "Search the library…",
            on_value_change: move |next| chosen.set(next),
            ComboboxEmpty { "Nothing in the library matches" }
            for (index, (id, text)) in choices.into_iter().enumerate() {
                ComboboxOption::<Option<ItemId>> {
                    key: "{id:?}",
                    index,
                    value: id,
                    text_value: text.clone(),
                    "{text}"
                }
            }
        }
    }
}
