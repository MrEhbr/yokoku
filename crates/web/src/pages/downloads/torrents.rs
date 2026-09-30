use dioxus::prelude::*;
use yokoku_domain::ImportId;

use crate::{
    api::{
        downloads::{DownloadEntry, DownloadState, ImportState, retry_import},
        failure,
    },
    components::{
        button::{Button, ButtonSize},
        progress::Progress,
        status::{Status, Tone},
        table::{Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow},
    },
    dialogs::import_review::ReviewButton,
    format::{plural, rate, runtime, size},
    route::Route,
};

#[component]
pub(super) fn Torrents(downloads: Vec<DownloadEntry>) -> Element {
    if downloads.is_empty() {
        return rsx! {
            p { class: "text-muted", "No downloads. Add a torrent to start one." }
        };
    }
    let count = downloads.len();
    rsx! {
        Table { class: "table-fixed",
            TableCaption { {plural(count, "torrent", "torrents")} }
            TableHeader {
                TableRow {
                    TableHead { "Name" }
                    TableHead { class: "w-48", "Item" }
                    TableHead { class: "w-20 text-right", "Size" }
                    TableHead { class: "w-56", "Progress" }
                    TableHead { class: "w-24 text-right", "Speed" }
                    TableHead { class: "w-24 text-right", "Left" }
                }
            }
            TableBody {
                for download in downloads {
                    Torrent { key: "{download.id}", download }
                }
            }
        }
    }
}

#[component]
fn Torrent(download: DownloadEntry) -> Element {
    let item = download.item.clone();
    let downloading = download.state == DownloadState::Downloading;
    rsx! {
        TableRow {
            TableCell { class: "truncate font-medium", title: "{download.name}", "{download.name}" }
            TableCell { class: "truncate",
                if let Some(item) = item {
                    Link {
                        class: "hover:underline",
                        title: "{item.title}",
                        to: Route::item(item.id),
                        "{item.title}"
                    }
                } else {
                    span { class: "text-muted", "—" }
                }
            }
            TableCell { class: "text-right tabular-nums whitespace-nowrap", "{size(download.size)}" }
            if downloading {
                TableCell {
                    div { class: "flex items-center gap-2",
                        Progress {
                            value: f64::from(download.percent),
                            aria_label: "{download.name} progress",
                        }
                        span { class: "w-9 shrink-0 text-right text-caption tabular-nums", "{download.percent}%" }
                    }
                }
                TableCell { class: "text-right tabular-nums whitespace-nowrap", "{rate(download.rate)}" }
                TableCell { class: "text-right tabular-nums whitespace-nowrap",
                    if let Some(eta) = download.eta {
                        "{runtime(eta.div_ceil(60))}"
                    }
                }
            } else {
                TableCell { colspan: 3,
                    State { download }
                }
            }
        }
    }
}

/// What happens to a torrent that is not downloading: the client's error first, then its import,
/// then the client's state.
#[component]
fn State(download: DownloadEntry) -> Element {
    let seeding = download.state == DownloadState::Seeding;
    let (tone, label, reason) = match (&download.state, &download.import) {
        (DownloadState::Error(error), _) => (Tone::Danger, "Error".to_owned(), Some(error.clone())),
        (_, Some(import)) => match &import.state {
            ImportState::NeedsReview => {
                return rsx! {
                    Action { tone: Tone::Warning, label: "Needs review", reason: None,
                        ReviewButton {
                            import: import.id,
                            label: "Review",
                            size: ButtonSize::Sm,
                            on_done: |()| {},
                        }
                    }
                };
            },
            ImportState::Failed(reason) => {
                return rsx! {
                    Action { tone: Tone::Danger, label: "Import failed", reason: Some(reason.clone()),
                        Retry { import: import.id }
                    }
                };
            },
            ImportState::Queued => (Tone::Muted, "Waiting to import".to_owned(), None),
            ImportState::Importing => (Tone::Info, "Importing…".to_owned(), None),
        },
        (_, None) if download.imported => {
            (Tone::Success, if seeding { "Imported · seeding" } else { "Imported" }.to_owned(), None)
        },
        (DownloadState::Paused, None) => (Tone::Muted, format!("Paused at {}%", download.percent), None),
        (state, None) => (Tone::Muted, state.label().to_owned(), None),
    };
    rsx! {
        Action { tone, label, reason }
    }
}

/// A status with its reason in full below it, and any action at the end of the row.
#[component]
fn Action(
    tone: Tone,
    label: String,
    #[props(!optional)] reason: Option<String>,
    #[props(default)] children: Element,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between gap-4",
            div { class: "min-w-0",
                Status { tone, label }
                if let Some(reason) = reason {
                    p { class: "mt-0.5 text-caption text-muted [overflow-wrap:anywhere]", "{reason}" }
                }
            }
            {children}
        }
    }
}

/// The row updates from the change the retry pushes.
#[component]
fn Retry(import: ImportId) -> Element {
    let mut busy = use_signal(|| false);
    let mut failed = use_signal(|| None::<String>);
    rsx! {
        div { class: "flex shrink-0 items-center gap-2",
            if let Some(error) = failed() {
                span { class: "text-caption text-danger", "{error}" }
            }
            Button {
                size: ButtonSize::Sm,
                disabled: busy(),
                aria_busy: busy(),
                onclick: move |_| async move {
                    busy.set(true);
                    match retry_import(import).await {
                        Ok(()) => failed.set(None),
                        Err(error) => failed.set(Some(failure(&error))),
                    }
                    busy.set(false);
                },
                "Retry"
            }
        }
    }
}
