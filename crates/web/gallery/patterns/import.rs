//! Manual import in a dialog. All editing happens in the browser; only loading and the import reach the server.

use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
    rc::Rc,
};

use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use jiff::Timestamp;
use yokoku_domain::{
    Artwork, Description, EpisodeMetadata, EpisodeSpan, ExternalId, ItemFolder, ItemName, MonitorPreset,
    SeasonMetadata, Series, SeriesMetadata, SourceStatus,
    naming::{Naming, NamingTemplates},
};
use yokoku_web::components::{
    button::{Button, ButtonSize, ButtonVariant},
    checkbox::{Checkbox, CheckboxState},
    dialog::{Dialog, DialogTitle},
    select::{Select, SelectOption},
    status::{Status, Tone},
    table::{SortDirection, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableSortHead},
};

use crate::patterns::backend::{FileRow, Match, SeriesInfo, import_files, library, pending_files, reset_demo};

#[component]
pub fn ManualImport() -> Element {
    let mut open = use_signal(|| false);
    let mut imported = use_signal(|| None::<usize>);

    rsx! {
        h1 { class: "yk-page-title", "Manual import" }
        p { class: "mt-2 max-w-prose text-muted",
            "Files added outside the app. Check rows, then set their series, season, or episodes from the menu \
             under the table. Episodes pair with the checked files in sort order."
        }
        if let Some(count) = imported() {
            p {
                role: "status",
                class: "mt-6 border border-info bg-info-soft px-3 py-2 text-info",
                "Imported {count} files."
            }
        }
        div { class: "mt-6 flex flex-wrap gap-2",
            Button {
                variant: ButtonVariant::Primary,
                onclick: move |_| open.set(true),
                "Import /downloads/complete"
            }
            Button {
                variant: ButtonVariant::Quiet,
                onclick: move |_| async move {
                    let _ = reset_demo().await;
                    imported.set(None);
                },
                "Reset the demo"
            }
        }
        Dialog { open: Some(open()), on_open_change: move |value| open.set(value),
            div { class: "flex items-start justify-between gap-4",
                DialogTitle { "Manual import · /downloads/complete" }
                Button {
                    variant: ButtonVariant::Quiet,
                    size: ButtonSize::Icon,
                    aria_label: "Close",
                    onclick: move |_| open.set(false),
                    X {}
                }
            }
            if open() {
                ImportLoader {
                    on_imported: move |count| {
                        imported.set(Some(count));
                        open.set(false);
                    },
                }
            }
        }
    }
}

#[component]
fn ImportLoader(on_imported: Callback<usize>) -> Element {
    let data = use_resource(|| async { Ok::<_, ServerFnError>((library().await?, pending_files().await?)) });
    let data = data.read();
    match &*data {
        None => rsx! {
            p { class: "text-muted", "Loading…" }
        },
        Some(Err(error)) => rsx! {
            p { class: "text-danger", "{error}" }
        },
        Some(Ok((library, files))) => rsx! {
            ImportEditor {
                library: library.clone(),
                files: files.clone(),
                on_imported,
            }
        },
    }
}

/// The library as the browser sees it, with the same naming engine the server uses.
struct Catalog {
    series: Vec<(SeriesInfo, Series)>,
    naming: Naming,
}

impl Catalog {
    fn new(library: &[SeriesInfo]) -> Self {
        let series = library.iter().map(|info| (info.clone(), to_series(info))).collect();
        let naming = Naming::new(&NamingTemplates::default()).expect("default templates parse");
        Self { series, naming }
    }

    fn info(&self, id: u64) -> Option<&SeriesInfo> {
        self.series.iter().map(|(info, _)| info).find(|info| info.id == id)
    }

    fn label(&self, id: u64) -> String {
        self.info(id).map_or_else(String::new, |info| ItemName::new(&info.title, info.year).to_string())
    }

    fn episode_count(&self, series: u64, season: u16) -> usize {
        self.info(series).and_then(|info| info.seasons.get(usize::from(season).checked_sub(1)?)).map_or(0, Vec::len)
    }

    fn episode_title(&self, series: u64, season: u16, episode: u16) -> String {
        let seasons = self.info(series).map(|info| &info.seasons);
        let title = seasons.and_then(|seasons| {
            seasons.get(usize::from(season).checked_sub(1)?)?.get(usize::from(episode).checked_sub(1)?)
        });
        title.cloned().unwrap_or_default()
    }

    /// The path a complete row imports to, relative to the series folder.
    fn target(&self, row: &FileRow) -> Option<String> {
        let (_, series) = self.series.iter().find(|(info, _)| Some(info.id) == row.series)?;
        let span = EpisodeSpan::new(row.season?, *row.episodes.first()?, *row.episodes.last()?)?;
        let extension = Path::new(&row.path).extension()?.to_str()?;
        let path = self.naming.episode_path(series, span, extension).ok()?;
        Some(path.to_string_lossy().into_owned())
    }
}

fn to_series(info: &SeriesInfo) -> Series {
    let mut source_id = 0;
    let seasons = (1..)
        .zip(&info.seasons)
        .map(|(number, titles)| SeasonMetadata {
            number,
            episodes: (1..)
                .zip(titles)
                .map(|(number, title)| {
                    source_id += 1;
                    EpisodeMetadata { source_id, number, title: title.clone(), overview: String::new(), air_date: None }
                })
                .collect(),
        })
        .collect();
    let metadata = SeriesMetadata {
        source: ExternalId::Tmdb(info.id),
        title: info.title.clone(),
        original_title: info.title.clone(),
        alternate_titles: Vec::new(),
        year: info.year,
        artwork: Artwork::default(),
        description: Description::default(),
        status: SourceStatus::Returning,
        seasons,
    };
    Series::new(
        metadata,
        ItemFolder::default(),
        MonitorPreset::All,
        jiff::civil::date(2026, 1, 1),
        Timestamp::UNIX_EPOCH,
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Key {
    File,
    Series,
    Season,
    Episode,
}

impl Key {
    const ALL: [(Self, &str); 4] =
        [(Self::File, "File"), (Self::Series, "Series"), (Self::Season, "Season"), (Self::Episode, "Episode")];
}

#[derive(Clone, Copy, PartialEq)]
struct Sort {
    key: Key,
    ascending: bool,
}

impl Sort {
    /// Sorts by `key`; pressing the sorted key again reverses it.
    fn press(&mut self, key: Key) {
        self.ascending = self.key != key || !self.ascending;
        self.key = key;
    }

    /// Unset values sort last in both directions.
    fn compare(self, a: &FileRow, b: &FileRow, catalog: &Catalog) -> Ordering {
        let directed = |ordering: Ordering| if self.ascending { ordering } else { ordering.reverse() };
        let unset_last = |a: bool, b: bool, ordering: Ordering| a.cmp(&b).then(directed(ordering));
        match self.key {
            Key::File => directed(natural(&a.path).cmp(&natural(&b.path))),
            Key::Series => {
                let title = |row: &FileRow| row.series.map(|id| catalog.label(id));
                unset_last(a.series.is_none(), b.series.is_none(), title(a).cmp(&title(b)))
            },
            Key::Season => unset_last(a.season.is_none(), b.season.is_none(), a.season.cmp(&b.season)),
            Key::Episode => {
                unset_last(a.episodes.is_empty(), b.episodes.is_empty(), a.episodes.first().cmp(&b.episodes.first()))
            },
        }
    }
}

/// A run of digits compares by value, so `engine_2` sorts before `engine_10`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Chunk {
    Number(u64),
    Text(String),
}

fn natural(name: &str) -> Vec<Chunk> {
    let lower = name.to_lowercase();
    let mut rest = lower.as_str();
    let mut chunks = Vec::new();
    while let Some(first) = rest.chars().next() {
        let digits = first.is_ascii_digit();
        let end = rest.find(|c: char| c.is_ascii_digit() != digits).unwrap_or(rest.len());
        let (chunk, tail) = rest.split_at(end);
        chunks.push(if digits {
            Chunk::Number(chunk.parse().unwrap_or(u64::MAX))
        } else {
            Chunk::Text(chunk.to_owned())
        });
        rest = tail;
    }
    chunks
}

#[derive(Clone, Copy, PartialEq)]
enum Panel {
    Series,
    Season,
    Episodes,
}

/// A row's state; a checked `blocking` row stops the import.
#[derive(Clone, PartialEq)]
struct Verdict {
    tone: Tone,
    label: &'static str,
    blocking: bool,
}

/// Duplicates count only among checked rows, since unchecked rows are not imported.
fn verdicts(rows: &[FileRow], checked: &HashSet<u64>) -> HashMap<u64, Verdict> {
    let mut claims: HashMap<(u64, u16, u16), usize> = HashMap::new();
    for row in rows.iter().filter(|row| checked.contains(&row.id)) {
        if let (Some(series), Some(season)) = (row.series, row.season) {
            for episode in &row.episodes {
                *claims.entry((series, season, *episode)).or_default() += 1;
            }
        }
    }
    let verdict = |tone, label, blocking| Verdict { tone, label, blocking };
    rows.iter()
        .map(|row| {
            let duplicate = row.episodes.iter().any(|episode| {
                let key = (row.series.unwrap_or_default(), row.season.unwrap_or_default(), *episode);
                claims.get(&key).is_some_and(|claims| *claims > 1)
            });
            let verdict = if row.series.is_none() {
                verdict(Tone::Warning, "Choose a series", true)
            } else if row.season.is_none() {
                verdict(Tone::Warning, "Choose a season", true)
            } else if row.episodes.is_empty() {
                verdict(Tone::Warning, "Choose episodes", true)
            } else if duplicate && checked.contains(&row.id) {
                verdict(Tone::Danger, "Another checked file has this episode", true)
            } else {
                verdict(Tone::Success, "Ready", false)
            };
            (row.id, verdict)
        })
        .collect()
}

#[component]
fn ImportEditor(library: Vec<SeriesInfo>, files: Vec<FileRow>, on_imported: Callback<usize>) -> Element {
    let catalog = use_hook(|| Rc::new(Catalog::new(&library)));
    let mut rows = use_signal(|| files.clone());
    let mut checked = use_signal(|| {
        let complete = |row: &&FileRow| row.series.is_some() && row.season.is_some() && !row.episodes.is_empty();
        files.iter().filter(complete).map(|row| row.id).collect::<HashSet<u64>>()
    });
    let mut sort = use_signal(|| Sort { key: Key::File, ascending: true });
    let mut panel = use_signal(|| None::<Panel>);
    let mut picked = use_signal(BTreeSet::<u16>::new);
    let mut error = use_signal(|| None::<String>);

    let sorted = use_memo({
        let catalog = catalog.clone();
        move || {
            let mut rows = rows();
            let sort = sort();
            rows.sort_by(|a, b| sort.compare(a, b, &catalog));
            rows
        }
    });
    let verdicts = use_memo(move || verdicts(&rows.read(), &checked.read()));
    let blocked = use_memo(move || checked.read().iter().filter(|id| verdicts.read()[*id].blocking).count());
    let common_season = use_memo(move || {
        let rows = rows.read();
        let checked = checked.read();
        let mut keys = rows.iter().filter(|row| checked.contains(&row.id)).map(|row| row.series.zip(row.season));
        let first = keys.next()??;
        keys.all(|key| key == Some(first)).then_some(first)
    });

    let total = rows.read().len();
    let checked_count = checked.read().len();
    let header_state = match checked_count {
        0 => CheckboxState::Unchecked,
        n if n == total => CheckboxState::Checked,
        _ => CheckboxState::Indeterminate,
    };
    // Checked rows in sort order receive the ticked episodes one each; a single row receives them all.
    let episode_plan = move || -> Option<Vec<(u64, Vec<u16>)>> {
        let ids: Vec<u64> = sorted.read().iter().map(|row| row.id).filter(|id| checked.read().contains(id)).collect();
        let episodes: Vec<u16> = picked.read().iter().copied().collect();
        match (ids.len(), episodes.len()) {
            (_, 0) | (0, _) => None,
            (1, _) => Some(vec![(ids[0], episodes)]),
            (rows, ticked) if rows == ticked => {
                Some(ids.into_iter().zip(episodes).map(|(id, episode)| (id, vec![episode])).collect())
            },
            _ => None,
        }
    };
    let mut update_checked = move |change: &dyn Fn(&mut FileRow)| {
        let ids = checked.read().clone();
        rows.write().iter_mut().filter(|row| ids.contains(&row.id)).for_each(change);
        panel.set(None);
        picked.write().clear();
    };
    let import = move |_| async move {
        let matches: Vec<Match> = rows
            .read()
            .iter()
            .filter(|row| checked.read().contains(&row.id))
            .filter_map(|row| {
                Some(Match { file: row.id, series: row.series?, season: row.season?, episodes: row.episodes.clone() })
            })
            .collect();
        match import_files(matches).await {
            Ok(count) => on_imported.call(count),
            Err(failure) => error.set(Some(failure.to_string())),
        }
    };

    rsx! {
        Table {
            TableHeader {
                TableRow {
                    TableHead {
                        Checkbox {
                            aria_label: "Select all files",
                            checked: Some(header_state),
                            on_checked_change: move |state| {
                                let ids = rows.read().iter().map(|row| row.id).collect();
                                checked.set(if state == CheckboxState::Checked { ids } else { HashSet::new() });
                            },
                        }
                    }
                    for (key, label) in Key::ALL {
                        TableSortHead {
                            key: "{label}",
                            label,
                            direction: (sort().key == key)
                                .then_some(
                                    if sort().ascending {
                                        SortDirection::Ascending
                                    } else {
                                        SortDirection::Descending
                                    },
                                ),
                            onclick: move |_| sort.write().press(key),
                        }
                    }
                }
            }
            TableBody {
                for row in sorted() {
                    TableRow {
                        key: "{row.id}",
                        "data-selected": checked.read().contains(&row.id),
                        TableCell {
                            Checkbox {
                                aria_label: "Select {row.path}",
                                checked: Some(
                                    if checked.read().contains(&row.id) {
                                        CheckboxState::Checked
                                    } else {
                                        CheckboxState::Unchecked
                                    },
                                ),
                                on_checked_change: move |state| {
                                    if state == CheckboxState::Checked {
                                        checked.write().insert(row.id);
                                    } else {
                                        checked.write().remove(&row.id);
                                    }
                                },
                            }
                        }
                        TableCell {
                            div { class: "grid gap-1",
                                span { class: "yk-code break-all", "{row.path}" }
                                if let Some(target) = catalog.target(&row) {
                                    span { class: "yk-code break-all text-muted", "→ {target}" }
                                }
                                Status {
                                    tone: verdicts.read()[&row.id].tone,
                                    label: verdicts.read()[&row.id].label,
                                }
                            }
                        }
                        TableCell { {row.series.map_or_else(|| "—".to_owned(), |id| catalog.label(id))} }
                        TableCell { class: "tabular-nums",
                            {row.season.map_or_else(|| "—".to_owned(), |season| season.to_string())}
                        }
                        TableCell { class: "whitespace-nowrap tabular-nums", {episode_codes(&row.episodes)} }
                    }
                }
            }
        }
        match panel() {
            Some(Panel::Series) => rsx! {
                section {
                    class: "flex flex-col gap-3 border border-line bg-subtle p-4",
                    aria_label: "Set series",
                    p { class: "text-caption text-muted", "Series for {checked_count} checked files" }
                    div { class: "flex flex-wrap gap-2",
                        for (info, _) in catalog.series.iter() {
                            Button {
                                key: "{info.id}",
                                onclick: {
                                    let id = info.id;
                                    move |_| update_checked(
                                        &|row: &mut FileRow| {
                                            if row.series != Some(id) {
                                                row.series = Some(id);
                                                row.season = None;
                                                row.episodes.clear();
                                            }
                                        },
                                    )
                                },
                                {catalog.label(info.id)}
                            }
                        }
                    }
                }
            },
            Some(Panel::Season) => rsx! {
                section {
                    class: "flex flex-col gap-3 border border-line bg-subtle p-4",
                    aria_label: "Set season",
                    p { class: "text-caption text-muted", "Series and season for {checked_count} checked files" }
                    for (info, _) in catalog.series.iter() {
                        div { key: "{info.id}", class: "flex flex-wrap items-center gap-2",
                            span { class: "w-48 text-caption", {catalog.label(info.id)} }
                            for season in 1..=info.seasons.len() as u16 {
                                Button {
                                    key: "{season}",
                                    onclick: {
                                        let id = info.id;
                                        move |_| update_checked(
                                            &|row: &mut FileRow| {
                                                if row.series.zip(row.season) != Some((id, season)) {
                                                    row.series = Some(id);
                                                    row.season = Some(season);
                                                    row.episodes.clear();
                                                }
                                            },
                                        )
                                    },
                                    "Season {season}"
                                }
                            }
                        }
                    }
                }
            },
            Some(Panel::Episodes) => rsx! {
                section {
                    class: "flex flex-col gap-3 border border-line bg-subtle p-4",
                    aria_label: "Set episodes",
                    if let Some((series, season)) = common_season() {
                        p { class: "text-caption text-muted", "{catalog.label(series)} · Season {season}" }
                        div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2",
                            for episode in 1..=catalog.episode_count(series, season) as u16 {
                                label {
                                    key: "{episode}",
                                    class: "flex items-center gap-2 font-mono text-caption",
                                    Checkbox {
                                        checked: Some(
                                            if picked.read().contains(&episode) {
                                                CheckboxState::Checked
                                            } else {
                                                CheckboxState::Unchecked
                                            },
                                        ),
                                        aria_label: "E{episode:02}",
                                        on_checked_change: move |state| {
                                            if state == CheckboxState::Checked {
                                                picked.write().insert(episode);
                                            } else {
                                                picked.write().remove(&episode);
                                            }
                                        },
                                    }
                                    "E{episode:02} · {catalog.episode_title(series, season, episode)}"
                                }
                            }
                        }
                        div { class: "flex flex-wrap items-center justify-end gap-3",
                            span { class: "mr-auto text-caption text-muted",
                                "{picked.read().len()} ticked for {checked_count} checked: one each, or all for a single file"
                            }
                            Button {
                                disabled: episode_plan().is_none(),
                                onclick: move |_| {
                                    let Some(plan) = episode_plan() else { return };
                                    for (id, episodes) in plan {
                                        rows.write()
                                            .iter_mut()
                                            .filter(|row| row.id == id)
                                            .for_each(|row| row.episodes.clone_from(&episodes));
                                    }
                                    panel.set(None);
                                    picked.write().clear();
                                },
                                "Assign episodes"
                            }
                        }
                    } else {
                        p { class: "text-caption text-muted", "Check files of one series and season first." }
                    }
                }
            },
            None => rsx! {},
        }
        if let Some(failure) = error() {
            p { role: "alert", class: "text-caption text-danger", "{failure}" }
        }
        div { class: "flex flex-wrap items-center gap-3 border-t border-line pt-4",
            div { class: "w-40",
                Select::<Panel> {
                    value: Some(panel.into()),
                    placeholder: "Set…",
                    disabled: checked_count == 0,
                    aria_label: "Set for checked files",
                    on_value_change: move |value| {
                        panel.set(value);
                        picked.write().clear();
                    },
                    SelectOption::<Panel> {
                        value: Panel::Series,
                        index: 0usize,
                        text_value: "Series",
                        "Series"
                    }
                    SelectOption::<Panel> {
                        value: Panel::Season,
                        index: 1usize,
                        text_value: "Season",
                        "Season"
                    }
                    SelectOption::<Panel> {
                        value: Panel::Episodes,
                        index: 2usize,
                        text_value: "Episodes",
                        "Episodes"
                    }
                }
            }
            span { class: "mr-auto text-caption text-muted", aria_live: "polite",
                "{checked_count} of {total} files checked"
                if blocked() > 0 {
                    " · {blocked} need attention"
                }
            }
            Button {
                variant: ButtonVariant::Primary,
                disabled: checked_count == 0 || blocked() > 0,
                onclick: import,
                "Import {checked_count} files"
            }
        }
    }
}

/// `E04` or `E04–E05`.
fn episode_codes(episodes: &[u16]) -> String {
    match (episodes.first(), episodes.last()) {
        (Some(first), Some(last)) if first == last => format!("E{first:02}"),
        (Some(first), Some(last)) => format!("E{first:02}–E{last:02}"),
        _ => "—".to_owned(),
    }
}
