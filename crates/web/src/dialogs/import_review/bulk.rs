use std::collections::BTreeSet;

use dioxus::prelude::*;
use yokoku_domain::{ImportId, SeriesId};

use crate::{
    api::{
        failure,
        review::{set_episodes, set_season, set_series},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        checkbox::{Checkbox, CheckboxState},
        label::Label,
        select::{Select, SelectOption},
        skeleton::Skeleton,
    },
    dialogs::pickers::{EpisodeChoice, SeasonPicker, SeriesPicker, episode_list},
};

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Series,
    Season,
    Episodes,
}

/// Changes every checked file at once: its series, its season keeping the episode numbers, or its
/// episodes ticked in order. `files` are their rows and paths, in table order; `series` is the one
/// series all of them have, if any, and `season` the season of the first.
#[component]
pub(super) fn BulkTools(
    import: ImportId,
    files: Vec<(usize, String)>,
    series: Option<SeriesId>,
    season: Option<u16>,
    on_change: Callback,
) -> Element {
    let mut tool = use_signal(|| None::<Tool>);
    let rows: Vec<usize> = files.iter().map(|(row, _)| *row).collect();
    let count = rows.len();
    let noun = if count == 1 { "file" } else { "files" };
    let mut pick = move |choice: Tool| tool.set(if tool() == Some(choice) { None } else { Some(choice) });
    let done = move |()| {
        tool.set(None);
        on_change(());
    };
    rsx! {
        div { class: "grid gap-3 border border-line bg-subtle p-3",
            div { class: "flex flex-wrap items-center gap-2",
                span { class: "mr-auto text-caption text-muted", "{count} checked {noun}" }
                for (choice, label) in [
                    (Tool::Series, "Set series…"),
                    (Tool::Season, "Set season…"),
                    (Tool::Episodes, "Set episodes…"),
                ]
                {
                    Button {
                        size: ButtonSize::Sm,
                        aria_pressed: tool() == Some(choice),
                        disabled: count == 0,
                        onclick: move |_| pick(choice),
                        "{label}"
                    }
                }
            }
            match tool() {
                Some(Tool::Series) => rsx! {
                    SeriesTool {
                        import,
                        rows,
                        series,
                        on_done: done,
                    }
                },
                Some(Tool::Season) => rsx! {
                    SeasonTool {
                        import,
                        rows,
                        series,
                        on_done: done,
                    }
                },
                Some(Tool::Episodes) => rsx! {
                    EpisodesTool {
                        import,
                        files,
                        series,
                        season,
                        on_done: done,
                    }
                },
                None => rsx! {},
            }
        }
    }
}

/// Detects the files again against the series chosen.
#[component]
fn SeriesTool(import: ImportId, rows: Vec<usize>, series: Option<SeriesId>, on_done: Callback) -> Element {
    let chosen = use_signal(|| series);
    let mut run = use_action(on_done);
    rsx! {
        SeriesPicker { id: "bulk-series", series: chosen }
        Apply {
            disabled: chosen().is_none(),
            busy: run.busy(),
            error: run.error(),
            label: "Detect with this series",
            onclick: move |_| {
                let rows = rows.clone();
                run.call(async move {
                    match chosen() {
                        Some(id) => set_series(import, rows, id).await,
                        None => Ok(()),
                    }
                })
            },
        }
    }
}

/// Places the files' episodes in the season chosen, keeping their numbers.
#[component]
fn SeasonTool(import: ImportId, rows: Vec<usize>, series: Option<SeriesId>, on_done: Callback) -> Element {
    let season = use_signal(|| None::<u16>);
    let mut run = use_action(on_done);
    let Some(series) = series else {
        return rsx! {
            p { class: "text-caption text-muted", "Give the checked files one series first." }
        };
    };
    rsx! {
        SeasonPicker {
            key: "{series}",
            id: "bulk-season",
            series,
            season,
            none: None,
        }
        Apply {
            disabled: season().is_none(),
            busy: run.busy(),
            error: run.error(),
            label: "Set this season",
            onclick: move |_| {
                let rows = rows.clone();
                run.call(async move {
                    match season() {
                        Some(season) => set_season(import, rows, season).await,
                        None => Ok(()),
                    }
                })
            },
        }
    }
}

/// Gives the files the episodes ticked, in order, one each; each ticked episode names the file it
/// goes to. Ticks stay while another season is listed, which starts at the files' `season`.
#[component]
fn EpisodesTool(
    import: ImportId,
    files: Vec<(usize, String)>,
    series: Option<SeriesId>,
    season: Option<u16>,
    on_done: Callback,
) -> Element {
    let mut ticked = use_signal(BTreeSet::<(u16, u16)>::new);
    let mut shown = use_signal(|| season);
    let mut run = use_action(on_done);
    let episodes = use_resource(move || async move {
        match series {
            Some(id) => episode_list(id).await,
            None => Vec::new(),
        }
    });
    let Some(series) = series else {
        return rsx! {
            p { class: "text-caption text-muted", "Give the checked files one series first." }
        };
    };
    let Some(episodes) = episodes.read().clone() else {
        return rsx! {
            Skeleton { class: "h-40 w-full" }
        };
    };
    let mut seasons: Vec<u16> = episodes.iter().map(|(season, _, _)| *season).collect();
    seasons.dedup();
    let listed = shown().filter(|season| seasons.contains(season)).or(seasons.first().copied());
    let in_season: Vec<EpisodeChoice> =
        episodes.iter().filter(|(season, _, _)| Some(*season) == listed).cloned().collect();
    let order: Vec<(u16, u16)> = ticked.read().iter().copied().collect();
    let file_for = |key: (u16, u16)| -> Option<String> {
        let position = order.iter().position(|ticked| *ticked == key)?;
        let path = &files.get(position)?.1;
        Some(path.rsplit('/').next().unwrap_or(path).to_owned())
    };
    let needed = files.len();
    let count = order.len();
    let season_name = |number: u16| if number == 0 { "Specials".to_owned() } else { format!("Season {number}") };
    let whole_season: Vec<(u16, u16)> = in_season.iter().map(|(season, number, _)| (*season, *number)).collect();
    let rows: Vec<usize> = files.iter().map(|(row, _)| *row).collect();
    let chosen = order.clone();
    rsx! {
        div { class: "flex flex-wrap items-end gap-2",
            div { class: "grid min-w-48 flex-1 gap-1.5",
                Label { html_for: "bulk-episodes-season", "Season" }
                Select::<u16> {
                    id: "bulk-episodes-season",
                    value: Some(shown.into()),
                    placeholder: listed.map(season_name).unwrap_or_default(),
                    on_value_change: move |next: Option<u16>| shown.set(next),
                    for (index, number) in seasons.iter().copied().enumerate() {
                        SelectOption::<u16> {
                            key: "{number}",
                            index,
                            value: number,
                            text_value: season_name(number),
                            {season_name(number)}
                        }
                    }
                }
            }
            Button { onclick: move |_| ticked.write().extend(whole_season.iter().copied()),
                "Tick all in season"
            }
            Button { onclick: move |_| ticked.write().clear(), "Clear" }
        }
        ul { class: "max-h-96 overflow-y-auto border border-line bg-surface",
            for (season, number, text) in in_season {
                li {
                    key: "{season}-{number}",
                    class: "border-b border-line last:border-b-0",
                    label {
                        r#for: "bulk-episode-{season}-{number}",
                        class: "flex cursor-pointer items-center gap-3 px-3 py-2.5 hover:bg-subtle",
                        Checkbox {
                            id: "bulk-episode-{season}-{number}",
                            checked: if ticked.read().contains(&(season, number)) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                            on_checked_change: move |state| {
                                if state == CheckboxState::Checked {
                                    ticked.write().insert((season, number));
                                } else {
                                    ticked.write().remove(&(season, number));
                                }
                            },
                        }
                        span { class: "min-w-0 flex-1", "{text}" }
                        match (ticked.read().contains(&(season, number)), file_for((season, number))) {
                            (true, Some(file)) => rsx! {
                                span { class: "yk-code min-w-0 truncate text-caption text-muted", "← {file}" }
                            },
                            (true, None) => rsx! {
                                span { class: "text-caption text-danger", "no file left" }
                            },
                            _ => rsx! {},
                        }
                    }
                }
            }
        }
        Apply {
            disabled: count != needed,
            busy: run.busy(),
            error: run.error(),
            label: "Set these episodes",
            note: format!("{count} of {needed} ticked; the checked files take them in order."),
            onclick: move |_| {
                let (rows, chosen) = (rows.clone(), chosen.clone());
                run.call(async move { set_episodes(import, rows, series, chosen).await })
            },
        }
    }
}

#[component]
fn Apply(
    disabled: bool,
    busy: bool,
    error: Option<String>,
    label: &'static str,
    #[props(default)] note: Option<String>,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-end gap-2",
            if let Some(message) = error {
                p { role: "alert", class: "mr-auto text-caption text-danger", "{message}" }
            } else if let Some(note) = note {
                p { class: "mr-auto text-caption text-muted", "{note}" }
            }
            Button {
                size: ButtonSize::Sm,
                variant: ButtonVariant::Primary,
                disabled: disabled || busy,
                aria_busy: busy,
                onclick,
                "{label}"
            }
        }
    }
}

/// A server call that calls `on_done` once it succeeds and keeps its failure to show.
#[derive(Clone, Copy)]
struct Action {
    busy: Signal<bool>,
    error: Signal<Option<String>>,
    on_done: Callback,
}

fn use_action(on_done: Callback) -> Action {
    Action { busy: use_signal(|| false), error: use_signal(|| None), on_done }
}

impl Action {
    fn busy(&self) -> bool {
        (self.busy)()
    }

    fn error(&self) -> Option<String> {
        (self.error)()
    }

    fn call(&mut self, call: impl Future<Output = Result<(), ServerFnError>> + 'static) {
        let mut action = *self;
        spawn(async move {
            action.busy.set(true);
            action.error.set(None);
            let result = call.await;
            action.busy.set(false);
            match result {
                Ok(()) => (action.on_done)(()),
                Err(failed) => action.error.set(Some(failure(&failed))),
            }
        });
    }
}
