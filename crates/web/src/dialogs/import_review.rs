use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use yokoku_domain::{ImportId, ItemId, title_with_year};

use crate::{
    api::{
        failure,
        library::{Entry, detail, library},
        review::{
            Confidence, Conflict, Imported, Match, ReviewFile, approve, match_file, replace_file, review, skip_file,
        },
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        combobox::{Combobox, ComboboxEmpty, ComboboxOption},
        dialog::{Dialog, DialogDescription, DialogFooter, DialogTitle},
        label::Label,
        skeleton::Skeleton,
        status::{Status, Tone},
    },
    format::{episode as code, size},
};

/// A button that opens the review of `import` (FR-4.11, 4.12, 8.3); `on_done` is called once
/// its files are imported.
#[component]
pub fn ReviewButton(
    import: ImportId,
    label: &'static str,
    #[props(default)] size: ButtonSize,
    on_done: Callback,
) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        Button { size, onclick: move |_| open.set(true), "{label}" }
        Dialog { open: Some(open()), on_open_change: move |next| open.set(next),
            div { class: "flex items-start justify-between gap-4",
                DialogTitle { "Review files" }
                Button {
                    variant: ButtonVariant::Quiet,
                    size: ButtonSize::Icon,
                    aria_label: "Close",
                    onclick: move |_| open.set(false),
                    X {}
                }
            }
            if open() {
                Rows {
                    import,
                    on_done: move |()| {
                        open.set(false);
                        on_done(());
                    },
                    on_close: move |()| open.set(false),
                }
            }
        }
    }
}

/// The import's files, read again after each change; changes are saved as they are made.
#[component]
fn Rows(import: ImportId, on_done: Callback, on_close: Callback) -> Element {
    let mut loaded = use_resource(move || review(import));
    let mut importing = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let reload = use_callback(move |()| loaded.restart());
    let current = loaded.read().clone();
    let Some(result) = current else {
        return rsx! {
            Skeleton { class: "h-48 w-full" }
        };
    };
    let review = match result {
        Ok(Some(review)) => review,
        Ok(None) => {
            return rsx! {
                p { class: "text-muted", "No files need review." }
                DialogFooter {
                    Button { onclick: move |_| on_close(()), "Close" }
                }
            };
        },
        Err(failed) => {
            return rsx! {
                p { role: "alert", class: "text-danger", {failure(&failed)} }
                DialogFooter {
                    Button { onclick: move |_| on_close(()), "Close" }
                }
            };
        },
    };
    let included: Vec<&ReviewFile> = review.rows.iter().filter(|row| !row.skipped).collect();
    let unmatched = included.iter().filter(|row| row.target.is_none()).count();
    let conflicting = included.iter().filter(|row| !row.conflicts.is_empty()).count();
    let count = included.len();
    let blocked = unmatched > 0 || conflicting > 0 || count == 0;
    let noun = if count == 1 { "file" } else { "files" };
    rsx! {
        DialogDescription {
            span { class: "yk-code [overflow-wrap:anywhere]", "{review.source}" }
            if review.from_download {
                " · the files are placed by the import mode in Settings."
            } else {
                " · the files are linked where they are."
            }
        }
        ul { class: "border-t border-line",
            for file in review.rows.clone() {
                FileRow {
                    key: "{file.row}",
                    import,
                    file,
                    from_download: review.from_download,
                    on_change: reload,
                }
            }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            span { class: "mr-auto text-caption text-muted",
                if unmatched > 0 {
                    "{unmatched} without a match. "
                }
                if conflicting > 0 {
                    "{conflicting} with a conflict. "
                }
                if blocked && count > 0 {
                    "Match or skip them to import."
                }
            }
            Button { onclick: move |_| on_close(()), "Close" }
            Button {
                variant: ButtonVariant::Primary,
                disabled: blocked || importing(),
                aria_busy: importing(),
                onclick: move |_| async move {
                    importing.set(true);
                    error.set(None);
                    match approve(import).await {
                        Ok(Imported::Linked(_) | Imported::Queued) => on_done(()),
                        Err(failed) => {
                            error.set(Some(failure(&failed)));
                            importing.set(false);
                        },
                    }
                },
                "Import {count} {noun}"
            }
        }
    }
}

/// A file with its match, confidence and conflicts, and what can be done with it.
#[component]
fn FileRow(import: ImportId, file: ReviewFile, from_download: bool, on_change: Callback) -> Element {
    let mut editing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let row = file.row;
    let (tone, label) = match file.confidence {
        Confidence::Certain => (Tone::Success, "Certain"),
        Confidence::Guess => (Tone::Warning, "Guess"),
        Confidence::Unknown => (Tone::Danger, "Unknown"),
    };
    let act = move |action: RowAction| async move {
        busy.set(true);
        error.set(None);
        let done = match action {
            RowAction::Skip => skip_file(import, row).await,
            RowAction::Replace => replace_file(import, row).await,
        };
        match done {
            Ok(()) => on_change(()),
            Err(failed) => error.set(Some(failure(&failed))),
        }
        busy.set(false);
    };
    let target = file.target.clone();
    let can_replace = from_download && !file.replace && file.conflicts.contains(&Conflict::AlreadyHasFile);
    rsx! {
        li { class: "grid gap-2 border-b border-line py-3",
            div { class: "flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1",
                span { class: "yk-code min-w-0 text-caption [overflow-wrap:anywhere]",
                    span { class: "text-muted", "{row}. " }
                    "{file.path}"
                }
                span { class: "flex shrink-0 items-center gap-3 text-caption text-muted",
                    "{size(file.size)}"
                    if !file.skipped {
                        Status { tone, label }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-x-3 gap-y-2",
                if file.skipped {
                    span { class: "text-muted", "Skipped; left where it is" }
                } else {
                    match &target {
                        Some(target) => rsx! {
                            span { "→ {target.label}" }
                        },
                        None => rsx! {
                            Status { tone: Tone::Warning, label: "No match" }
                        },
                    }
                    if file.replace {
                        span { class: "text-caption text-warning", "replaces the library file" }
                    }
                    for conflict in file.conflicts.clone() {
                        span { class: "text-caption text-danger", "{conflict.label()}" }
                    }
                }
                span { class: "ml-auto flex flex-wrap gap-2",
                    Button {
                        size: ButtonSize::Sm,
                        disabled: busy(),
                        onclick: move |_| editing.toggle(),
                        if file.skipped {
                            "Include…"
                        } else if file.target.is_some() {
                            "Change match"
                        } else {
                            "Match…"
                        }
                    }
                    if can_replace {
                        Button {
                            size: ButtonSize::Sm,
                            variant: ButtonVariant::Danger,
                            disabled: busy(),
                            title: "The library file is deleted when this one is imported",
                            onclick: move |_| act(RowAction::Replace),
                            "Replace library file"
                        }
                    }
                    if !file.skipped {
                        Button {
                            size: ButtonSize::Sm,
                            variant: ButtonVariant::Quiet,
                            disabled: busy(),
                            onclick: move |_| act(RowAction::Skip),
                            "Skip"
                        }
                    }
                }
            }
            if let Some(message) = error() {
                p { role: "alert", class: "text-caption text-danger", "{message}" }
            }
            if editing() {
                MatchEditor {
                    import,
                    row,
                    current: file.target.map(|target| target.matched),
                    on_saved: move |()| {
                        editing.set(false);
                        on_change(());
                    },
                    on_cancel: move |()| editing.set(false),
                }
            }
        }
    }
}

/// Picks the series and episodes, or the movie, a file holds.
#[component]
fn MatchEditor(
    import: ImportId,
    row: usize,
    current: Option<Match>,
    on_saved: Callback,
    on_cancel: Callback,
) -> Element {
    let items = use_resource(|| library(None, None, None));
    let mut item = use_signal(|| current.map(Match::item));
    let mut episodes = use_signal(|| match current {
        Some(Match::Episodes { season, first, last, .. }) => Some((season, first, last)),
        _ => None,
    });
    let mut series = use_resource(move || async move {
        match item() {
            Some(ItemId::Series(id)) => detail::series(id).await.ok().flatten(),
            _ => None,
        }
    });
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let chosen = match (item(), episodes()) {
        (Some(ItemId::Series(series)), Some((season, first, last))) => {
            Some(Match::Episodes { series, season, first, last })
        },
        (Some(ItemId::Movie(id)), _) => Some(Match::Movie { id }),
        _ => None,
    };
    let entries: Vec<Entry> = match &*items.read() {
        Some(Ok(entries)) => entries.clone(),
        _ => Vec::new(),
    };
    let item_choice = use_memo(move || Some(item()));
    let episode_list: Vec<(u16, u16, String)> = series
        .read()
        .clone()
        .flatten()
        .map(|series| {
            series
                .seasons
                .iter()
                .flat_map(|season| season.episodes.iter())
                .map(|episode| {
                    (
                        episode.season,
                        episode.number,
                        format!("{} {}", code(episode.season, episode.number), episode.title),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let first_choice = use_memo(move || episodes().map(|(season, first, _)| (season, first)));
    let last_choice = use_memo(move || episodes().map(|(season, _, last)| (season, last)));
    let later: Vec<(u16, u16, String)> = match episodes() {
        Some((season, first, _)) => {
            episode_list.iter().filter(|(s, n, _)| *s == season && *n >= first).cloned().collect()
        },
        None => Vec::new(),
    };
    let id = format!("match-{row}");
    rsx! {
        div { class: "grid gap-3 border border-line bg-subtle p-3",
            div { class: "grid gap-1.5",
                Label { html_for: "{id}-item", "Series or movie" }
                if items.read().is_none() {
                    Skeleton { class: "h-9 w-full" }
                } else {
                    Combobox::<Option<ItemId>> {
                        id: "{id}-item",
                        value: Some(item_choice.into()),
                        placeholder: "Search the library…",
                        on_value_change: move |next: Option<Option<ItemId>>| {
                            item.set(next.flatten());
                            series.clear();
                            episodes.set(None);
                        },
                        ComboboxEmpty { "Nothing in the library matches" }
                        for (index, entry) in entries.iter().enumerate() {
                            ComboboxOption::<Option<ItemId>> {
                                key: "{entry.id:?}",
                                index,
                                value: Some(entry.id),
                                text_value: format!("{} · {}", title_with_year(&entry.title, entry.year), entry.id.kind()),
                                "{title_with_year(&entry.title, entry.year)} · {entry.id.kind()}"
                            }
                        }
                    }
                }
            }
            if matches!(item(), Some(ItemId::Series(_))) {
                if series.read().is_none() {
                    Skeleton { class: "h-9 w-full" }
                } else {
                    div { class: "grid gap-3 sm:grid-cols-2",
                        div { class: "grid gap-1.5",
                            Label { html_for: "{id}-first", "Episode" }
                            Combobox::<(u16, u16)> {
                                id: "{id}-first",
                                value: Some(first_choice.into()),
                                placeholder: "Search by code or title…",
                                on_value_change: move |next: Option<(u16, u16)>| {
                                    episodes.set(next.map(|(season, number)| (season, number, number)));
                                },
                                ComboboxEmpty { "No episode matches" }
                                for (index, (season, number, text)) in episode_list.iter().cloned().enumerate() {
                                    ComboboxOption::<(u16, u16)> {
                                        key: "{season}-{number}",
                                        index,
                                        value: (season, number),
                                        text_value: text.clone(),
                                        "{text}"
                                    }
                                }
                            }
                        }
                        div { class: "grid gap-1.5",
                            Label { html_for: "{id}-last", "Through" }
                            Combobox::<(u16, u16)> {
                                id: "{id}-last",
                                value: Some(last_choice.into()),
                                placeholder: "Search by code or title…",
                                disabled: episodes().is_none(),
                                on_value_change: move |next: Option<(u16, u16)>| {
                                    if let (Some((_, last)), Some((season, first, _))) = (next, episodes()) {
                                        episodes.set(Some((season, first, last)));
                                    }
                                },
                                ComboboxEmpty { "No episode matches" }
                                for (index, (season, number, text)) in later.iter().cloned().enumerate() {
                                    ComboboxOption::<(u16, u16)> {
                                        key: "{season}-{number}",
                                        index,
                                        value: (season, number),
                                        text_value: text.clone(),
                                        "{text}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(message) = error() {
                p { role: "alert", class: "text-caption text-danger", "{message}" }
            }
            div { class: "flex justify-end gap-2",
                Button { size: ButtonSize::Sm, onclick: move |_| on_cancel(()), "Cancel" }
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Primary,
                    disabled: chosen.is_none() || saving(),
                    aria_busy: saving(),
                    onclick: move |_| async move {
                        let Some(target) = chosen else { return };
                        saving.set(true);
                        error.set(None);
                        match match_file(import, row, target).await {
                            Ok(()) => on_saved(()),
                            Err(failed) => {
                                error.set(Some(failure(&failed)));
                                saving.set(false);
                            },
                        }
                    },
                    "Save match"
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum RowAction {
    Skip,
    Replace,
}
