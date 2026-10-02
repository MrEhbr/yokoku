mod bulk;

use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use yokoku_domain::{ImportId, ItemId, ItemName, SeriesId};

use self::bulk::BulkTools;
use crate::{
    api::{
        failure,
        library::{Entry, detail, library},
        review::{
            Conflict, Imported, Match, Resolution, ReviewFile, approve, include_files, keep_both_file, match_file,
            replace_file, review, set_episodes, set_season,
        },
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        checkbox::{Checkbox, CheckboxState},
        combobox::{Combobox, ComboboxEmpty, ComboboxOption},
        dialog::{DialogDescription, DialogFooter},
        label::Label,
        select::{Select, SelectOption},
        skeleton::Skeleton,
        table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    },
    dialogs::{
        ClosableDialog,
        pickers::{EpisodePicker, episode_list},
    },
    format::{episode as code, plural, size},
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
        ClosableDialog { title: "Review files", open, wide: true,
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

/// The import's files, read again after each change; changes are saved as they are made. Checked
/// files are imported and get the bulk tools.
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
    let checked: Vec<&ReviewFile> = review.rows.iter().filter(|file| file.included).collect();
    let chosen: Vec<(usize, String)> = checked.iter().map(|file| (file.row, file.path.clone())).collect();
    let all: Vec<usize> = review.rows.iter().map(|file| file.row).collect();
    let series = match checked.iter().map(|file| file.item.as_ref().map(|item| item.id)).collect::<Vec<_>>()[..] {
        [Some(ItemId::Series(first)), ref rest @ ..] if rest.iter().all(|id| *id == Some(ItemId::Series(first))) => {
            Some(first)
        },
        _ => None,
    };
    let unmatched = checked.iter().filter(|file| file.problem.is_some()).count();
    let conflicting = checked.iter().filter(|file| !file.conflicts.is_empty()).count();
    let count = checked.len();
    let blocked = unmatched > 0 || conflicting > 0 || count == 0;
    let files = plural(count, "file", "files");
    let header = match count {
        0 => CheckboxState::Unchecked,
        picked if picked == all.len() => CheckboxState::Checked,
        _ => CheckboxState::Indeterminate,
    };
    rsx! {
        DialogDescription {
            span { class: "yk-code [overflow-wrap:anywhere]", "{review.source}" }
            if review.from_download {
                " · checked files are placed by the import mode in Settings; the others stay where they are."
            } else {
                " · checked files are linked where they are."
            }
        }
        BulkTools {
            import,
            files: chosen,
            series,
            season: checked.first().and_then(|file| file.season),
            on_change: reload,
        }
        Table {
            TableHeader {
                TableRow {
                    TableHead { class: "w-8",
                        Checkbox {
                            aria_label: "Check every file",
                            checked: header,
                            on_checked_change: move |_| {
                                let all = all.clone();
                                let check = header != CheckboxState::Checked;
                                async move {
                                    error.set(None);
                                    match include_files(import, all, check).await {
                                        Ok(()) => reload(()),
                                        Err(failed) => error.set(Some(failure(&failed))),
                                    }
                                }
                            },
                        }
                    }
                    TableHead { "File" }
                    TableHead { "Series" }
                    TableHead { "Season" }
                    TableHead { "Episodes" }
                    TableHead { class: "text-right", "Size" }
                    TableHead {
                        span { class: "sr-only", "Actions" }
                    }
                }
            }
            TableBody {
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
        }
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger", "{message}" }
        }
        DialogFooter {
            span { class: "mr-auto text-caption text-muted",
                if unmatched > 0 {
                    "{unmatched} checked without a match. "
                }
                if conflicting > 0 {
                    "{conflicting} with a conflict. "
                }
                if blocked && count > 0 {
                    "Fix or uncheck them to import."
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
                        }
                    }
                },
                "Import {files}"
            }
        }
    }
}

/// A file with its match and what it lacks, and what can be done with it; its editor opens in a
/// row below.
#[component]
fn FileRow(import: ImportId, file: ReviewFile, from_download: bool, on_change: Callback) -> Element {
    let mut editing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let row = file.row;
    let act = move |action: RowAction| async move {
        busy.set(true);
        error.set(None);
        let done = match action {
            RowAction::Include(included) => include_files(import, vec![row], included).await,
            RowAction::Replace => replace_file(import, row).await,
            RowAction::KeepBoth => keep_both_file(import, row).await,
        };
        match done {
            Ok(()) => on_change(()),
            Err(failed) => error.set(Some(failure(&failed))),
        }
        busy.set(false);
    };
    let can_replace = from_download && file.conflicts.contains(&Conflict::AlreadyHasFile);
    let can_keep_both = !file.conflicts.is_empty();
    let mut cell = use_signal(|| None::<Cell>);
    let series = match file.item.as_ref().map(|item| item.id) {
        Some(ItemId::Series(id)) => Some(id),
        _ => None,
    };
    let blank = || {
        rsx! {
            span { class: "text-muted", "—" }
        }
    };
    let season_text = match file.season {
        Some(0) => rsx! { "Specials" },
        Some(season) => rsx! { "{season}" },
        None => blank(),
    };
    let episodes_text = match file.episodes.clone() {
        Some(episodes) => rsx! { "{episodes}" },
        None => blank(),
    };
    rsx! {
        TableRow { "data-selected": file.included,
            TableCell { class: "align-top",
                Checkbox {
                    id: "review-row-{row}",
                    aria_label: "Import {file.path}",
                    disabled: busy(),
                    checked: if file.included { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: move |state| act(RowAction::Include(state == CheckboxState::Checked)),
                }
            }
            TableCell { class: "min-w-64 align-top",
                label {
                    r#for: "review-row-{row}",
                    class: "yk-code block cursor-pointer text-caption [overflow-wrap:anywhere]",
                    "{file.path}"
                }
                div { class: "mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-caption",
                    if let Some(problem) = file.problem.clone() {
                        span { class: if file.included { "text-danger" } else { "text-muted" },
                            "{problem}"
                        }
                    } else if file.guessed {
                        span { class: "text-warning", "Guessed from the name" }
                    }
                    if file.included {
                        for conflict in file.conflicts.clone() {
                            span { class: "text-danger", "{conflict.label()}" }
                        }
                        match file.resolution {
                            Resolution::Replace => rsx! {
                                span { class: "text-warning", "Replaces the library file" }
                            },
                            Resolution::KeepBoth => rsx! {
                                span { class: "text-muted", "Kept beside the other files" }
                            },
                            Resolution::Unresolved => rsx! {},
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
                        if can_keep_both {
                            Button {
                                size: ButtonSize::Sm,
                                disabled: busy(),
                                title: if from_download { "Both are kept; this one gets a numbered name where its own is taken" } else { "Both are kept, each where it is" },
                                onclick: move |_| act(RowAction::KeepBoth),
                                "Keep both"
                            }
                        }
                    }
                }
                if let Some(name) = file.name.clone() {
                    p { class: "mt-1 text-caption text-muted",
                        "New name "
                        span { class: "yk-code text-ink [overflow-wrap:anywhere]", "{name}" }
                    }
                }
                if let Some(message) = error() {
                    p { role: "alert", class: "mt-1 text-caption text-danger", "{message}" }
                }
            }
            TableCell { class: "align-top",
                match file.item.clone() {
                    Some(item) => rsx! { "{item.title}" },
                    None => blank(),
                }
            }
            TableCell { class: "align-top tabular-nums",
                match (series, cell()) {
                    (Some(series), Some(Cell::Season)) => rsx! {
                        SeasonCell {
                            import,
                            row,
                            series,
                            on_done: move |()| {
                                cell.set(None);
                                on_change(());
                            },
                            on_cancel: move |()| cell.set(None),
                        }
                    },
                    (Some(_), _) => rsx! {
                        CellButton {
                            label: "Change the season of {file.path}",
                            onclick: move |_| cell.set(Some(Cell::Season)),
                            {season_text}
                        }
                    },
                    (None, _) => season_text,
                }
            }
            TableCell { class: "align-top",
                match (series, cell()) {
                    (Some(series), Some(Cell::Episodes)) => rsx! {
                        EpisodeCell {
                            import,
                            row,
                            series,
                            on_done: move |()| {
                                cell.set(None);
                                on_change(());
                            },
                            on_cancel: move |()| cell.set(None),
                        }
                    },
                    (Some(_), _) => rsx! {
                        CellButton {
                            label: "Change the episode of {file.path}",
                            onclick: move |_| cell.set(Some(Cell::Episodes)),
                            {episodes_text}
                        }
                    },
                    (None, _) => episodes_text,
                }
            }
            TableCell { class: "align-top text-right whitespace-nowrap text-muted", "{size(file.size)}" }
            TableCell { class: "align-top text-right",
                Button {
                    size: ButtonSize::Sm,
                    variant: ButtonVariant::Quiet,
                    disabled: busy(),
                    aria_expanded: editing(),
                    onclick: move |_| editing.toggle(),
                    "Edit"
                }
            }
        }
        if editing() {
            TableRow {
                TableCell { colspan: 7,
                    MatchEditor {
                        import,
                        row,
                        current: file.matched,
                        suggested: file.item.as_ref().map(|item| item.id),
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
}

/// Picks the series and episodes, or the movie, a file holds, starting from its match or the
/// `suggested` item.
#[component]
fn MatchEditor(
    import: ImportId,
    row: usize,
    current: Option<Match>,
    suggested: Option<ItemId>,
    on_saved: Callback,
    on_cancel: Callback,
) -> Element {
    let items = use_resource(|| library(None, None, None, None));
    let mut item = use_signal(|| current.map(Match::item).or(suggested));
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
        .iter()
        .flatten()
        .flat_map(|series| &series.seasons)
        .flat_map(|season| &season.episodes)
        .map(|episode| {
            (episode.season, episode.number, format!("{} {}", code(episode.season, episode.number), episode.title))
        })
        .collect();
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
                                text_value: format!("{} · {}", ItemName::new(&entry.title, entry.year), entry.id.kind()),
                                "{ItemName::new(&entry.title, entry.year)} · {entry.id.kind()}"
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
                            Combobox::<(u16,u16)> {
                                id: "{id}-first",
                                value: Some(first_choice.into()),
                                placeholder: "Search by code or title…",
                                on_value_change: move |next: Option<(u16, u16)>| {
                                    episodes.set(next.map(|(season, number)| (season, number, number)));
                                },
                                ComboboxEmpty { "No episode matches" }
                                for (index, (season, number, text)) in episode_list.iter().cloned().enumerate() {
                                    ComboboxOption::<(u16,u16)> {
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
                            Combobox::<(u16,u16)> {
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
                                    ComboboxOption::<(u16,u16)> {
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
                            }
                        }
                    },
                    "Save match"
                }
            }
        }
    }
}

/// The table cell being edited in place.
#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Season,
    Episodes,
}

/// A cell's value that opens its editor when clicked.
#[component]
fn CellButton(label: String, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx! {
        button {
            r#type: "button",
            aria_label: "{label}",
            class: "cursor-pointer text-left underline decoration-line decoration-dotted underline-offset-4 hover:decoration-ink",
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}

/// Picks one season for the file in place; it keeps its episode numbers.
#[component]
fn SeasonCell(import: ImportId, row: usize, series: SeriesId, on_done: Callback, on_cancel: Callback) -> Element {
    let mut error = use_signal(|| None::<String>);
    let seasons = use_resource(move || async move {
        let detail = detail::series(series).await.ok().flatten();
        let mut numbers: Vec<u16> =
            detail.map(|detail| detail.seasons.iter().map(|season| season.number).collect()).unwrap_or_default();
        numbers.sort_by_key(|&number| (number == 0, number));
        numbers
    });
    let Some(numbers) = seasons.read().clone() else {
        return rsx! {
            Skeleton { class: "h-9 w-32" }
        };
    };
    let name = |number: u16| if number == 0 { "Specials".to_owned() } else { format!("Season {number}") };
    rsx! {
        div { class: "flex min-w-40 items-center gap-1",
            Select::<u16> {
                aria_label: "Season",
                placeholder: "Season…",
                on_value_change: move |next: Option<u16>| {
                    if let Some(season) = next {
                        spawn(async move {
                            match set_season(import, vec![row], season).await {
                                Ok(()) => on_done(()),
                                Err(failed) => error.set(Some(failure(&failed))),
                            }
                        });
                    }
                },
                for (index, number) in numbers.into_iter().enumerate() {
                    SelectOption::<u16> {
                        key: "{number}",
                        index,
                        value: number,
                        text_value: name(number),
                        {name(number)}
                    }
                }
            }
            CancelButton { on_cancel }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "mt-1 text-caption text-danger", "{message}" }
        }
    }
}

/// Picks one episode for the file in place.
#[component]
fn EpisodeCell(import: ImportId, row: usize, series: SeriesId, on_done: Callback, on_cancel: Callback) -> Element {
    let episode = use_signal(|| None::<(u16, u16)>);
    let mut error = use_signal(|| None::<String>);
    let episodes = use_resource(move || episode_list(series));
    use_effect(move || {
        if let Some(chosen) = episode() {
            spawn(async move {
                match set_episodes(import, vec![row], series, vec![chosen]).await {
                    Ok(()) => on_done(()),
                    Err(failed) => error.set(Some(failure(&failed))),
                }
            });
        }
    });
    let Some(episodes) = episodes.read().clone() else {
        return rsx! {
            Skeleton { class: "h-9 w-56" }
        };
    };
    rsx! {
        div { class: "flex min-w-72 items-center gap-1",
            div { class: "min-w-0 flex-1",
                EpisodePicker {
                    id: format!("review-row-{row}-episode"),
                    label: "Episode",
                    hide_label: true,
                    episodes,
                    episode,
                }
            }
            CancelButton { on_cancel }
        }
        if let Some(message) = error() {
            p { role: "alert", class: "mt-1 text-caption text-danger", "{message}" }
        }
    }
}

#[component]
fn CancelButton(on_cancel: Callback) -> Element {
    rsx! {
        Button {
            variant: ButtonVariant::Quiet,
            size: ButtonSize::Icon,
            aria_label: "Cancel",
            onclick: move |_| on_cancel(()),
            X {}
        }
    }
}

#[derive(Clone, Copy)]
enum RowAction {
    Include(bool),
    Replace,
    KeepBoth,
}
