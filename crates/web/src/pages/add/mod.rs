mod dialog;
mod hit;

use dioxus::prelude::*;

use self::{
    dialog::AddDialog,
    hit::{HitRow, HitSkeleton},
};
use crate::{
    api::{
        add::{SearchHit, add_options, search},
        failure,
        library::Kind,
    },
    components::{
        button::{Button, ButtonVariant},
        input::Input,
    },
    layout::BackButton,
    route::{Route, SearchText},
};

/// Searches the metadata source for series or movies to add (FR-1.1); the search is in the URL.
#[component]
pub fn Add(query: SearchText, kind: Kind) -> Element {
    let mut text = use_signal(|| query.0.clone());
    use_effect(use_reactive!(|query| text.set(query.0)));
    let options = use_resource(add_options);
    let picked = use_signal(|| None::<SearchHit>);
    let navigator = navigator();

    rsx! {
        document::Title { "Add · Yokoku" }
        BackButton { fallback: Route::Library {} }
        h1 { class: "yk-page-title mt-4", "Add" }
        form {
            role: "search",
            class: "mt-6 flex flex-wrap gap-2",
            onsubmit: move |event| {
                event.prevent_default();
                let query = SearchText(text().trim().to_owned());
                navigator.push(Route::Add { query, kind });
            },
            div { role: "group", aria_label: "Kind", class: "flex",
                for option in Kind::ALL {
                    Button {
                        key: "{option}",
                        r#type: "button",
                        class: "-ml-px text-muted first:ml-0 aria-pressed:z-10 aria-pressed:border-ink aria-pressed:bg-subtle aria-pressed:text-ink",
                        variant: ButtonVariant::Secondary,
                        aria_pressed: kind == option,
                        onclick: {
                            let query = query.clone();
                            move |_| {
                                navigator
                                    .replace(Route::Add {
                                        query: query.clone(),
                                        kind: option,
                                    });
                            }
                        },
                        match option {
                            Kind::Series => "Series",
                            Kind::Movie => "Movies",
                        }
                    }
                }
            }
            Input {
                r#type: "search",
                class: "min-w-40 flex-1",
                aria_label: "Title",
                placeholder: match kind {
                    Kind::Series => "Title of a series",
                    Kind::Movie => "Title of a movie",
                },
                value: "{text}",
                oninput: move |event: FormEvent| text.set(event.value()),
            }
            Button { r#type: "submit", "Search" }
        }
        div { class: "mt-6",
            SuspenseBoundary {
                fallback: |_| rsx! {
                    Loading {}
                },
                Results { query, kind, picked }
            }
        }
        AddDialog { picked, options }
    }
}

#[component]
fn Loading() -> Element {
    rsx! {
        ul {
            aria_busy: "true",
            aria_label: "Loading results",
            class: "grid divide-y divide-line border-y border-line",
            for index in 0..4 {
                HitSkeleton { key: "{index}" }
            }
        }
    }
}

/// The hits for `query`; picking one opens the add dialog.
#[component]
fn Results(query: SearchText, kind: Kind, picked: Signal<Option<SearchHit>>) -> Element {
    let searched = query.0.clone();
    let results = use_server_future(use_reactive!(|(query, kind)| async move {
        let text = query.0.trim().to_owned();
        if text.is_empty() {
            return Ok(None);
        }
        search(text, kind).await.map(Some)
    }))?;
    let what = match kind {
        Kind::Series => "series",
        Kind::Movie => "movies",
    };

    rsx! {
        match &*results.read() {
            None => rsx! {
                Loading {}
            },
            Some(Err(error)) => rsx! {
                p { role: "alert", class: "text-danger", {failure(error)} }
            },
            Some(Ok(None)) => rsx! {
                p { class: "text-muted", "Search {what} by title." }
            },
            Some(Ok(Some(hits))) if hits.is_empty() => rsx! {
                p { class: "text-muted", "No {what} found for “{searched}”." }
            },
            Some(Ok(Some(hits))) => rsx! {
                ul {
                    aria_label: "Results",
                    class: "grid divide-y divide-line border-y border-line",
                    for hit in hits.iter().cloned() {
                        HitRow {
                            key: "{hit.source}",
                            hit,
                            on_pick: move |hit| picked.set(Some(hit)),
                        }
                    }
                }
            },
        }
    }
}
