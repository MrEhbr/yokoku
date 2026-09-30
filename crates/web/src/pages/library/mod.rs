mod fields;
mod filters;
mod grid;
mod table;

use dioxus::prelude::*;
use dioxus_icons::lucide::{LayoutGrid, Plus, Rows3};

use self::{
    filters::{FilterBar, Filters},
    grid::PosterGrid,
    table::EntryTable,
};
use crate::{
    api::library::library,
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        button::{Button, ButtonSize, ButtonVariant},
        skeleton::Skeleton,
    },
    route::{Route, SearchText},
};

#[derive(Clone, Copy, PartialEq)]
enum View {
    Grid,
    Table,
}

/// Movies and series with type and status filters, a sort order, and a poster or table view (FR-1.2, 1.3).
#[component]
pub fn Library() -> Element {
    let mut filters = use_signal(Filters::default);
    let mut view = use_signal(|| View::Grid);
    let entries = use_server_future(move || {
        let Filters { kind, status, sort } = filters();
        library(kind, status, Some(sort))
    })?;

    rsx! {
        document::Title { "Library · Yokoku" }
        div { class: "flex flex-wrap items-center justify-between gap-4",
            h1 { class: "yk-page-title", "Library" }
            Link {
                class: "yk-button yk-button-primary ml-auto [&>svg]:size-4",
                to: Route::Add {
                    query: SearchText::default(),
                    kind: filters().kind.unwrap_or_default(),
                },
                Plus {}
                "Add"
            }
            div { role: "group", aria_label: "View", class: "flex gap-1",
                for (option, label, icon) in [
                    (View::Grid, "Posters", rsx! {
                        LayoutGrid {}
                    }),
                    (View::Table, "Table", rsx! {
                        Rows3 {}
                    }),
                ]
                {
                    Button {
                        key: "{label}",
                        variant: ButtonVariant::Quiet,
                        size: ButtonSize::Icon,
                        class: "aria-pressed:bg-subtle aria-pressed:text-ink",
                        aria_label: label,
                        aria_pressed: view() == option,
                        onclick: move |_| view.set(option),
                        {icon}
                    }
                }
            }
        }
        FilterBar { filters }
        div { class: "mt-8",
            match &*entries.read() {
                None => rsx! {
                    Skeleton { class: "h-64 w-full" }
                },
                Some(Err(_)) => rsx! {
                    Alert { variant: AlertVariant::Danger,
                        AlertTitle { "The library could not be loaded" }
                        AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
                    }
                },
                Some(Ok(entries)) if entries.is_empty() && filters().narrows() => rsx! {
                    div { class: "flex flex-col items-start gap-3",
                        p { class: "text-muted", "No items match these filters." }
                        Button {
                            onclick: move |_| filters.write().clear(),
                            "Clear filters"
                        }
                    }
                },
                Some(Ok(entries)) if entries.is_empty() => rsx! {
                    p { class: "text-muted", "Your library is empty. Add a movie or series to start." }
                },
                Some(Ok(entries)) => {
                    match view() {
                        View::Grid => rsx! {
                            PosterGrid { entries: entries.clone() }
                        },
                        View::Table => rsx! {
                            EntryTable { entries: entries.clone() }
                        },
                    }
                }
            }
        }
    }
}
