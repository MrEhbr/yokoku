use dioxus::prelude::*;
use jiff::civil::Date;

use crate::{
    api::library::calendar::{AgendaEntry, AgendaRelease},
    components::item_status::FileState,
    format::{episode as code, relative},
    route::Route,
};

/// Entries under a heading per day, in date order.
#[component]
pub(super) fn AgendaList(entries: Vec<AgendaEntry>, today: Date) -> Element {
    let mut days: Vec<(Date, Vec<AgendaEntry>)> = Vec::new();
    for entry in entries {
        match days.last_mut() {
            Some((day, group)) if *day == entry.date => group.push(entry),
            _ => days.push((entry.date, vec![entry])),
        }
    }
    rsx! {
        div { class: "flex flex-col gap-8",
            for (day, group) in days {
                section { key: "{day}",
                    h2 { class: "text-section font-medium",
                        if day.year() == today.year() {
                            "{day.strftime(\"%A, %b %-d\")}"
                        } else {
                            "{day.strftime(\"%A, %b %-d, %Y\")}"
                        }
                        span {
                            class: "ml-2 text-caption font-medium",
                            class: if day == today { "text-info" } else { "text-muted" },
                            "{relative(day, today)}"
                        }
                    }
                    ul { class: "mt-3 border-t border-line",
                        for (index, entry) in group.into_iter().enumerate() {
                            li {
                                key: "{index}",
                                class: "flex flex-wrap items-baseline gap-x-4 gap-y-1 border-b border-line px-3 py-3",
                                Link {
                                    class: "font-medium hover:underline",
                                    to: Route::item(entry.item),
                                    "{entry.title}"
                                }
                                Release { release: entry.release }
                                span { class: "sm:ml-auto",
                                    FileState { status: entry.status }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Release(release: AgendaRelease) -> Element {
    match release {
        AgendaRelease::Episode { season, number, title } => rsx! {
            span { class: "text-muted",
                span { class: "yk-code mr-2", "{code(season, number)}" }
                "{title}"
            }
        },
        AgendaRelease::Movie(kind) => rsx! {
            span { class: "text-muted", "{kind.label()} release" }
        },
    }
}
