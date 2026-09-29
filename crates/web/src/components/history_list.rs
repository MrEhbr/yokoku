use dioxus::prelude::*;
use jiff::civil::Date;
use yokoku_domain::ItemId;

use crate::{
    api::history::{HistoryEntry, HistoryPage, Part, history, movie_history, series_history},
    components::button::{Button, ButtonSize},
    format::relative,
    route::Route,
};

/// Whose history a `HistoryList` shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HistoryScope {
    Library,
    Item(ItemId),
}

async fn load(scope: HistoryScope, before: Option<i64>) -> Result<HistoryPage, ServerFnError> {
    match scope {
        HistoryScope::Library => history(before).await,
        HistoryScope::Item(ItemId::Series(id)) => series_history(id, before).await,
        HistoryScope::Item(ItemId::Movie(id)) => movie_history(id, before).await,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Older {
    Idle,
    Loading,
    Failed,
}

/// Events newest first under a label per day, a page at a time. Give it a `key` of its scope,
/// so older pages are dropped when the scope changes.
#[component]
pub fn HistoryList(scope: HistoryScope, empty: &'static str) -> Element {
    use_context_provider(|| scope);
    let first = use_server_future(use_reactive!(|scope| load(scope, None)))?;
    let mut more = use_signal(Vec::<HistoryEntry>::new);
    let mut cursor = use_signal(|| None::<Option<i64>>);
    let mut older = use_signal(|| Older::Idle);

    let first = first.read();
    let page = match &*first {
        None => return rsx! {},
        Some(Err(_)) => {
            return rsx! {
                p { class: "text-muted", "The history could not be loaded." }
            };
        },
        Some(Ok(page)) => page,
    };
    if page.entries.is_empty() {
        return rsx! {
            p { class: "text-muted", "{empty}" }
        };
    }
    let next = cursor().unwrap_or(page.older);
    let days = by_day(page.entries.iter().chain(more.read().iter()).cloned());
    let today = page.today;

    rsx! {
        div { class: "flex flex-col gap-6",
            for (day, entries) in days {
                Day { key: "{day}", day, today, entries }
            }
        }
        if let Some(before) = next {
            div { class: "mt-4 flex items-center gap-3",
                Button {
                    size: ButtonSize::Sm,
                    disabled: older() == Older::Loading,
                    aria_busy: older() == Older::Loading,
                    onclick: move |_| async move {
                        older.set(Older::Loading);
                        match load(scope, Some(before)).await {
                            Ok(page) => {
                                more.write().extend(page.entries);
                                cursor.set(Some(page.older));
                                older.set(Older::Idle);
                            },
                            Err(_) => older.set(Older::Failed),
                        }
                    },
                    "Show older"
                }
                if older() == Older::Failed {
                    span { class: "text-caption text-danger", "Older entries could not be loaded." }
                }
            }
        }
    }
}

fn by_day(entries: impl Iterator<Item = HistoryEntry>) -> Vec<(Date, Vec<HistoryEntry>)> {
    let mut days: Vec<(Date, Vec<HistoryEntry>)> = Vec::new();
    for entry in entries {
        match days.last_mut() {
            Some((day, group)) if *day == entry.at.date() => group.push(entry),
            _ => days.push((entry.at.date(), vec![entry])),
        }
    }
    days
}

#[component]
fn Day(day: Date, today: Date, entries: Vec<HistoryEntry>) -> Element {
    let label = if day.year() == today.year() {
        day.strftime("%A, %b %-d").to_string()
    } else {
        day.strftime("%A, %b %-d, %Y").to_string()
    };
    rsx! {
        section { aria_label: "{label}",
            p { class: "text-caption font-medium",
                "{label}"
                span { class: "ml-2 text-muted", "{relative(day, today)}" }
            }
            ul { class: "mt-2 border-t border-line",
                for entry in entries {
                    Entry { key: "{entry.id}", entry }
                }
            }
        }
    }
}

#[component]
fn Entry(entry: HistoryEntry) -> Element {
    let time = entry.at.strftime("%H:%M").to_string();
    rsx! {
        li { class: "flex gap-4 border-b border-line py-2.5 [overflow-wrap:anywhere]",
            time {
                class: "w-11 shrink-0 text-muted tabular-nums",
                datetime: "{entry.at}",
                "{time}"
            }
            div { class: "min-w-0 flex-1",
                p {
                    Parts { parts: entry.summary }
                }
                Details { lines: entry.details }
            }
        }
    }
}

/// A single line shows as is; several fold under their count.
#[component]
fn Details(lines: Vec<Vec<Part>>) -> Element {
    let class = "text-caption text-muted";
    match lines.len() {
        0 => rsx! {},
        1 => rsx! {
            p { class,
                Parts { parts: lines[0].clone() }
            }
        },
        count => rsx! {
            details { class: "group mt-0.5",
                summary { class: "cursor-pointer text-caption text-muted hover:text-ink",
                    span { class: "group-open:hidden", "Show {count} files" }
                    span { class: "hidden group-open:inline", "Hide files" }
                }
                ul { class: "mt-1 flex flex-col gap-0.5",
                    for (index, parts) in lines.into_iter().enumerate() {
                        li { key: "{index}", class,
                            Parts { parts }
                        }
                    }
                }
            }
        },
    }
}

/// Items link to their pages, except the item whose page this is and removed ones.
#[component]
fn Parts(parts: Vec<Part>) -> Element {
    let scope = use_context::<HistoryScope>();
    rsx! {
        for (index, part) in parts.into_iter().enumerate() {
            match part {
                Part::Text(text) => rsx! {
                    span { key: "{index}", "{text}" }
                },
                Part::Item { id: Some(id), title } if scope != HistoryScope::Item(id) => rsx! {
                    Link {
                        key: "{index}",
                        class: "font-medium underline decoration-muted underline-offset-4 hover:decoration-current",
                        to: Route::item(id),
                        "{title}"
                    }
                },
                Part::Item { title, .. } => rsx! {
                    span { key: "{index}", class: "font-medium", "{title}" }
                },
                Part::Code(code) => rsx! {
                    span { key: "{index}", class: "yk-code", "{code}" }
                },
                Part::Path(path) => rsx! {
                    span { key: "{index}", class: "font-mono", "{path}" }
                },
            }
        }
    }
}
