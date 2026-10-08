use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use yokoku_domain::{ItemId, ItemName, MonitorPreset};

use crate::{
    api::{
        add::{AddOptions, NewItem, RootChoice, SearchHit, add_item},
        failure,
        library::Kind,
    },
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        button::{Button, ButtonSize, ButtonVariant},
        dialog::{Dialog, DialogFooter, DialogTitle},
        field::{Field, FieldError, FieldHint},
        input::Input,
        label::Label,
        select::{Select, SelectOption},
        skeleton::Skeleton,
    },
    route::{Route, SettingsPart},
};

/// The options of the picked result; adding it opens its page. Closing it
/// clears `picked`.
#[component]
pub(crate) fn AddDialog(
    picked: Signal<Option<SearchHit>>,
    options: Resource<Result<AddOptions, ServerFnError>>,
    #[props(default)] on_added: Option<Callback<ItemId>>,
) -> Element {
    let open = picked.read().is_some();
    rsx! {
        Dialog {
            open: Some(open),
            on_open_change: move |next: bool| {
                if !next {
                    picked.set(None);
                }
            },
            if let Some(hit) = picked() {
                div { class: "flex items-start justify-between gap-4",
                    DialogTitle { "Add {ItemName::new(&hit.title, hit.year)}" }
                    Button {
                        variant: ButtonVariant::Quiet,
                        size: ButtonSize::Icon,
                        aria_label: "Close",
                        onclick: move |_| picked.set(None),
                        X {}
                    }
                }
                Options {
                    hit,
                    options,
                    on_close: move |()| picked.set(None),
                    on_added,
                }
            }
        }
    }
}

#[component]
fn Options(
    hit: SearchHit,
    options: Resource<Result<AddOptions, ServerFnError>>,
    on_close: Callback,
    on_added: Option<Callback<ItemId>>,
) -> Element {
    rsx! {
        match &*options.read() {
            None => rsx! {
                Skeleton { class: "h-48 w-full" }
                DialogFooter {
                    Button { onclick: move |_| on_close(()), "Cancel" }
                }
            },
            Some(Err(error)) => rsx! {
                p { role: "alert", class: "text-danger", {failure(error)} }
                DialogFooter {
                    Button { onclick: move |_| on_close(()), "Cancel" }
                }
            },
            Some(Ok(options)) => rsx! {
                OptionsForm {
                    roots: options.roots(hit.kind).to_vec(),
                    hit,
                    monitor: options.monitor,
                    on_close,
                    on_added,
                }
            },
        }
    }
}

/// `monitor` is the default preset.
#[component]
fn OptionsForm(
    hit: SearchHit,
    roots: Vec<RootChoice>,
    monitor: MonitorPreset,
    on_close: Callback,
    on_added: Option<Callback<ItemId>>,
) -> Element {
    let default_monitor = monitor;
    let root = use_signal(|| roots.first().map(|root| root.path.clone()));
    let monitor = use_signal(|| {
        Some(match (hit.kind, default_monitor) {
            (Kind::Movie, MonitorPreset::None) => MonitorPreset::None,
            (Kind::Movie, _) => MonitorPreset::All,
            (Kind::Series, preset) => preset,
        })
    });
    let folder = use_signal(|| hit.folder.clone());
    let mut adding = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let kind_name = match hit.kind {
        Kind::Series => "series",
        Kind::Movie => "movies",
    };
    let presets: &'static [(MonitorPreset, &str, &str)] = match hit.kind {
        Kind::Series => &[
            (MonitorPreset::All, "All episodes", "Every episode, aired or not, counts toward Wanted and Upcoming."),
            (MonitorPreset::Future, "Future episodes", "Only episodes that have not aired yet."),
            (MonitorPreset::LatestSeason, "Latest season", "Only the episodes of the latest season."),
            (MonitorPreset::None, "Nothing", "No episode shows in Wanted or Upcoming."),
        ],
        Kind::Movie => &[
            (MonitorPreset::All, "Monitored", "Shows in Wanted and Upcoming until it has a file."),
            (MonitorPreset::None, "Not monitored", "Never shows in Wanted or Upcoming."),
        ],
    };
    let monitor_hint = presets.iter().find(|(preset, ..)| Some(*preset) == monitor()).map(|(.., hint)| *hint);
    let path = format!("{}/{}", root().unwrap_or_default(), folder().trim());
    let chosen = roots.iter().find(|choice| Some(&choice.path) == root.read().as_ref());
    let has = |names: fn(&RootChoice) -> &[String]| {
        chosen.is_some_and(|choice| names(choice).iter().any(|name| name == folder().trim()))
    };
    let (taken, existing) = (has(|choice| &choice.taken), has(|choice| &choice.folders));

    if roots.is_empty() {
        return rsx! {
            Alert { variant: AlertVariant::Warning,
                AlertTitle { "No root folder for {kind_name}" }
                AlertDescription {
                    "Add one in "
                    Link {
                        class: "underline",
                        to: Route::Settings {
                            part: SettingsPart::RootFolders,
                        },
                        "Settings"
                    }
                    ", then try again."
                }
            }
            DialogFooter {
                Button { onclick: move |_| on_close(()), "Cancel" }
            }
        };
    }

    let (kind, source) = (hit.kind, hit.source.clone());
    let submit = move |_| {
        let source = source.clone();
        async move {
            let (Some(root), Some(monitor)) = (root(), monitor()) else { return };
            adding.set(true);
            let item = NewItem { kind, source, root, monitor, folder: folder() };
            match add_item(item).await {
                Ok(id) => {
                    if let Some(on_added) = on_added {
                        on_added(id);
                        on_close(());
                    } else {
                        navigator().push(Route::item(id));
                    }
                },
                Err(failed) => {
                    error.set(Some(failure(&failed)));
                    adding.set(false);
                },
            }
        }
    };
    rsx! {
        div { class: "flex flex-col gap-5 sm:flex-row sm:items-start",
            if let Some(poster) = &hit.poster {
                img {
                    class: "hidden aspect-[2/3] w-28 shrink-0 border border-ink bg-subtle object-cover shadow-paper sm:block",
                    src: "{poster}",
                    alt: "",
                }
            }
            div { class: "grid min-w-0 flex-1 gap-4",
                OptionFields {
                    roots,
                    root,
                    presets,
                    monitor,
                    monitor_hint: monitor_hint.unwrap_or_default(),
                    folder,
                    folder_taken: taken,
                    folder_hint: if taken { format!("{path} belongs to another item; choose another name.") } else if existing { format!("{path} exists; the files in it are linked to this item.") } else { format!("Creates {path}.") },
                }
            }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            Button { onclick: move |_| on_close(()), "Cancel" }
            Button {
                variant: ButtonVariant::Primary,
                disabled: adding() || taken || folder().trim().is_empty(),
                aria_busy: adding(),
                onclick: submit,
                "Add"
            }
        }
    }
}

#[component]
fn OptionFields(
    roots: Vec<RootChoice>,
    root: Signal<Option<String>>,
    presets: &'static [(MonitorPreset, &'static str, &'static str)],
    monitor: Signal<Option<MonitorPreset>>,
    monitor_hint: &'static str,
    folder: Signal<String>,
    folder_taken: bool,
    folder_hint: String,
) -> Element {
    rsx! {
        Field {
            Label { html_for: "add-root", "Root folder" }
            Select::<String> {
                id: "add-root",
                value: Some(root.into()),
                on_value_change: move |next| root.set(next),
                for (index, RootChoice { path, name, .. }) in roots.into_iter().enumerate() {
                    SelectOption::<String> {
                        key: "{path}",
                        index,
                        value: path.clone(),
                        text_value: name.clone(),
                        "{name}"
                    }
                }
            }
        }
        Field {
            Label { html_for: "add-monitor", "Monitor" }
            Select::<MonitorPreset> {
                id: "add-monitor",
                value: Some(monitor.into()),
                aria_describedby: "add-monitor-hint",
                on_value_change: move |next| monitor.set(next),
                for (index, (preset, label, _)) in presets.iter().enumerate() {
                    SelectOption::<MonitorPreset> {
                        key: "{label}",
                        index,
                        value: *preset,
                        text_value: *label,
                        "{label}"
                    }
                }
            }
            FieldHint { id: "add-monitor-hint", "{monitor_hint}" }
        }
        Field {
            Label { html_for: "add-folder", "Folder" }
            Input {
                id: "add-folder",
                value: "{folder}",
                aria_describedby: "add-folder-hint",
                aria_invalid: if folder_taken { "true" } else { "false" },
                oninput: move |event: FormEvent| folder.set(event.value()),
            }
            if folder_taken {
                FieldError { id: "add-folder-hint", class: "break-words", "{folder_hint}" }
            } else {
                FieldHint { id: "add-folder-hint", class: "break-words", "{folder_hint}" }
            }
        }
    }
}
