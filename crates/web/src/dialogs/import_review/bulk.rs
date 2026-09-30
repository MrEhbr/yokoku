use dioxus::prelude::*;
use yokoku_domain::{ImportId, SeriesId};

use crate::{
    api::{
        failure,
        library::detail,
        review::{Match, ReviewFile, match_file, redetect_files},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        label::Label,
        select::{Select, SelectOption},
    },
    dialogs::pickers::{SeasonPicker, SeriesPicker},
    format::episode as code,
};

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Series,
    InOrder,
}

/// Detects the selected files again, against the library or a chosen series and season, or
/// assigns them episodes in order after a preview; `on_change` is called once they changed.
#[component]
pub(super) fn BulkTools(import: ImportId, files: Vec<ReviewFile>, on_change: Callback) -> Element {
    let mut tool = use_signal(|| None::<Tool>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let rows: Vec<usize> = files.iter().map(|file| file.row).collect();
    let count = rows.len();
    let noun = if count == 1 { "file" } else { "files" };
    let detect_again = {
        let rows = rows.clone();
        move |_| {
            let rows = rows.clone();
            async move {
                busy.set(true);
                error.set(None);
                match redetect_files(import, rows, None, None).await {
                    Ok(()) => on_change(()),
                    Err(failed) => error.set(Some(failure(&failed))),
                }
                busy.set(false);
            }
        }
    };
    let mut pick = move |choice: Tool| tool.set(if tool() == Some(choice) { None } else { Some(choice) });
    rsx! {
        div { class: "grid gap-3 border border-line bg-subtle p-3",
            div { class: "flex flex-wrap items-center gap-2",
                span { class: "mr-auto text-caption text-muted", "{count} {noun} selected" }
                Button {
                    size: ButtonSize::Sm,
                    aria_pressed: tool() == Some(Tool::Series),
                    disabled: busy(),
                    onclick: move |_| pick(Tool::Series),
                    "Match to a series…"
                }
                Button {
                    size: ButtonSize::Sm,
                    aria_pressed: tool() == Some(Tool::InOrder),
                    disabled: busy(),
                    onclick: move |_| pick(Tool::InOrder),
                    "Assign in order…"
                }
                Button {
                    size: ButtonSize::Sm,
                    disabled: busy(),
                    aria_busy: busy() && tool().is_none(),
                    onclick: detect_again,
                    "Detect again"
                }
            }
            match tool() {
                Some(Tool::Series) => rsx! {
                    SeriesMatch { import, rows: rows.clone(), on_change }
                },
                Some(Tool::InOrder) => rsx! {
                    InOrder { import, files: files.clone(), on_change }
                },
                None => rsx! {},
            }
            if let Some(message) = error() {
                p { role: "alert", class: "text-caption text-danger", "{message}" }
            }
        }
    }
}

/// Detects the files again against a series, their names without a season placed in the season
/// chosen, if any.
#[component]
fn SeriesMatch(import: ImportId, rows: Vec<usize>, on_change: Callback) -> Element {
    let series = use_signal(|| None::<SeriesId>);
    let season = use_signal(|| None::<u16>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let apply = move |_| {
        let rows = rows.clone();
        async move {
            let Some(chosen) = series() else { return };
            busy.set(true);
            error.set(None);
            match redetect_files(import, rows, Some(chosen), season()).await {
                Ok(()) => on_change(()),
                Err(failed) => {
                    error.set(Some(failure(&failed)));
                    busy.set(false);
                },
            }
        }
    };
    rsx! {
        div { class: "grid gap-3 sm:grid-cols-2",
            SeriesPicker { id: "bulk-series", series }
            if let Some(id) = series() {
                SeasonPicker {
                    key: "{id}",
                    id: "bulk-season",
                    series: id,
                    season,
                    none: Some("From the file names"),
                }
            }
        }
        div { class: "flex items-center justify-end gap-2",
            if let Some(message) = error() {
                p { role: "alert", class: "mr-auto text-caption text-danger", "{message}" }
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Primary,
                disabled: series().is_none() || busy(),
                aria_busy: busy(),
                onclick: apply,
                "Detect with this series"
            }
        }
    }
}

/// Gives the files consecutive episodes of one season from a starting episode, in the order
/// shown, after previewing every mapping.
#[component]
fn InOrder(import: ImportId, files: Vec<ReviewFile>, on_change: Callback) -> Element {
    let series = use_signal(|| None::<SeriesId>);
    let season = use_signal(|| None::<u16>);
    let mut first = use_signal(|| None::<u16>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    use_effect(move || {
        season();
        first.set(None);
    });
    let detail = use_resource(move || async move {
        match series() {
            Some(id) => detail::series(id).await.ok().flatten(),
            None => None,
        }
    });
    let episodes: Vec<(u16, String)> = detail
        .read()
        .clone()
        .flatten()
        .and_then(|detail| detail.seasons.into_iter().find(|found| Some(found.number) == season()))
        .map(|found| found.episodes.into_iter().map(|episode| (episode.number, episode.title)).collect())
        .unwrap_or_default();
    let start = first().and_then(|first| episodes.iter().position(|(number, _)| *number == first));
    let preview: Vec<(ReviewFile, Option<(u16, String)>)> = match start {
        Some(start) => files
            .iter()
            .cloned()
            .enumerate()
            .map(|(offset, file)| (file, episodes.get(start + offset).cloned()))
            .collect(),
        None => Vec::new(),
    };
    let short = preview.iter().filter(|(_, episode)| episode.is_none()).count();
    let assignments: Vec<(usize, Match)> = match (series(), season()) {
        (Some(id), Some(season)) => preview
            .iter()
            .filter_map(|(file, episode)| {
                let (number, _) = episode.as_ref()?;
                Some((file.row, Match::Episodes { series: id, season, first: *number, last: *number }))
            })
            .collect(),
        _ => Vec::new(),
    };
    let ready = !preview.is_empty() && short == 0;
    let apply = move |_| {
        let assignments = assignments.clone();
        async move {
            busy.set(true);
            error.set(None);
            for (row, target) in assignments {
                if let Err(failed) = match_file(import, row, target).await {
                    error.set(Some(failure(&failed)));
                    busy.set(false);
                    on_change(());
                    return;
                }
            }
            on_change(());
        }
    };
    let episode_options: Vec<(u16, String)> = episodes.clone();
    rsx! {
        div { class: "grid gap-3 sm:grid-cols-3",
            SeriesPicker { id: "order-series", series }
            if let Some(id) = series() {
                SeasonPicker { key: "{id}", id: "order-season", series: id, season, none: None }
            }
            if season().is_some() && !episode_options.is_empty() {
                div { class: "grid gap-1.5",
                    Label { html_for: "order-first", "Starting at" }
                    Select::<u16> {
                        key: "{season:?}",
                        id: "order-first",
                        value: Some(first.into()),
                        placeholder: "Choose…",
                        on_value_change: move |next| first.set(next),
                        for (index, (number, title)) in episode_options.into_iter().enumerate() {
                            SelectOption::<u16> {
                                key: "{number}",
                                index,
                                value: number,
                                text_value: format!("E{number:02} {title}"),
                                "E{number:02} {title}"
                            }
                        }
                    }
                }
            }
        }
        if !preview.is_empty() {
            ol { class: "grid gap-1 text-caption", aria_label: "Preview",
                for (file, episode) in preview {
                    li { key: "{file.row}", class: "flex flex-wrap gap-x-2",
                        span { class: "yk-code text-muted [overflow-wrap:anywhere]", "{file.path}" }
                        span { aria_hidden: "true", "→" }
                        match (episode, season()) {
                            (Some((number, title)), Some(season)) => rsx! {
                                span { "{code(season, number)} {title}" }
                            },
                            _ => rsx! {
                                span { class: "text-danger", "no episode left in the season" }
                            },
                        }
                    }
                }
            }
        }
        div { class: "flex items-center justify-end gap-2",
            if let Some(message) = error() {
                p { role: "alert", class: "mr-auto text-caption text-danger", "{message}" }
            } else if short > 0 {
                p { class: "mr-auto text-caption text-danger",
                    "The season has too few episodes from there; start earlier or select fewer files."
                }
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Primary,
                disabled: !ready || busy(),
                aria_busy: busy(),
                onclick: apply,
                "Assign these episodes"
            }
        }
    }
}
