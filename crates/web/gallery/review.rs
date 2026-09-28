//! The import review screen (FR-4.11, 4.12) against in-memory state: per-row correction, selection, conflicts.

use std::{collections::HashMap, sync::Mutex};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        error::{SeeOther, bad_request, see_other},
        href, page, query_params, route,
    },
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, view},
};
use yokoku_web::components::{
    rename_row::*,
    selection_bar::*,
    status::*,
    ui::{alert::*, button::*, checkbox::*, select::*},
};

use crate::{story, story_page};

const SERIES: &str = "Orbital (2024)";
const EPISODES: [(u32, &str); 6] =
    [(1, "Launch"), (2, "Drift"), (3, "Burn"), (4, "Coast"), (5, "Descent"), (6, "Landfall")];
/// Episodes the library already has a file for.
const EXISTING: [u32; 1] = [3];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Confidence {
    Certain,
    Guess,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Resolution {
    Replace,
    Skip,
    KeepBoth,
}

impl Resolution {
    const ALL: [Self; 3] = [Self::Replace, Self::Skip, Self::KeepBoth];

    fn value(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Skip => "skip",
            Self::KeepBoth => "keep-both",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Replace => "Replace",
            Self::Skip => "Skip this file",
            Self::KeepBoth => "Keep both",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|resolution| resolution.value() == value)
    }
}

#[derive(Clone)]
struct Row {
    id: u64,
    file: &'static str,
    confidence: Confidence,
    episode: Option<u32>,
    selected: bool,
    resolution: Option<Resolution>,
}

#[derive(Clone, Copy)]
enum Conflict {
    Duplicate,
    Existing,
}

/// A row's state as the reviewer sees it; `blocking` states stop the import.
struct Verdict {
    tone: Tone,
    label: String,
    blocking: bool,
}

/// The pending import plan, shared by every request.
pub(crate) struct Review {
    rows: Mutex<Vec<Row>>,
}

impl Review {
    pub(crate) fn new() -> Self {
        let row =
            |id, file, confidence, episode| Row { id, file, confidence, episode, selected: false, resolution: None };
        Self {
            rows: Mutex::new(vec![
                row(1, "orbital.s01e01.1080p.mkv", Confidence::Certain, Some(1)),
                row(2, "orbital.s01e02.1080p.mkv", Confidence::Certain, Some(2)),
                row(3, "[Group] Orbital - 03 [1080p].mkv", Confidence::Guess, Some(3)),
                row(4, "orbital.ep4.mkv", Confidence::Guess, Some(4)),
                row(5, "orbital.ep04.proper.mkv", Confidence::Guess, Some(4)),
                row(6, "orbital.bonus.mkv", Confidence::Unknown, None),
                row(7, "orbital.s01e06.1080p.mkv", Confidence::Certain, Some(6)),
            ]),
        }
    }

    fn rows(&self) -> Vec<Row> {
        self.rows.lock().unwrap().clone()
    }

    /// Applies `change` to every row `matches` picks; returns how many it changed.
    fn update(&self, matches: impl Fn(&Row) -> bool, change: impl FnMut(&mut Row)) -> usize {
        let mut rows = self.rows.lock().unwrap();
        rows.iter_mut().filter(|row| matches(row)).map(change).count()
    }
}

/// Rows that share an episode with another imported file, or target an episode that
/// already has one. Skipped rows claim no episode but keep their conflict.
fn conflicts(rows: &[Row]) -> HashMap<u64, Conflict> {
    let imported = |row: &Row| row.resolution != Some(Resolution::Skip);
    let mut claims: HashMap<u32, usize> = HashMap::new();
    for episode in rows.iter().filter(|row| imported(row)).filter_map(|row| row.episode) {
        *claims.entry(episode).or_default() += 1;
    }
    rows.iter()
        .filter_map(|row| {
            let episode = row.episode?;
            let others = claims.get(&episode).copied().unwrap_or_default() - usize::from(imported(row));
            if others > 0 {
                Some((row.id, Conflict::Duplicate))
            } else if EXISTING.contains(&episode) {
                Some((row.id, Conflict::Existing))
            } else {
                None
            }
        })
        .collect()
}

/// The name a row imports as, or `None` when it has no episode or is skipped.
fn target(row: &Row) -> Option<String> {
    let episode = row.episode?;
    let (_, title) = EPISODES.iter().find(|(number, _)| *number == episode)?;
    let suffix = match row.resolution {
        Some(Resolution::Skip) => return None,
        Some(Resolution::KeepBoth) => " (2)",
        _ => "",
    };
    Some(format!("{SERIES} - S01E{episode:02} - {title}{suffix}.mkv"))
}

fn verdicts(rows: &[Row]) -> HashMap<u64, Verdict> {
    let conflicts = conflicts(rows);
    let mut names: HashMap<String, usize> = HashMap::new();
    for name in rows.iter().filter_map(target) {
        *names.entry(name).or_default() += 1;
    }
    let verdict = |tone, label: &str, blocking| Verdict { tone, label: label.to_owned(), blocking };
    rows.iter()
        .map(|row| {
            let conflict = conflicts.get(&row.id).copied();
            let verdict = match (row.episode, conflict, row.resolution) {
                (_, _, Some(Resolution::Skip)) => verdict(Tone::Muted, "Skipped", false),
                (None, _, _) => verdict(Tone::Warning, "Choose an episode", true),
                (_, Some(Conflict::Duplicate), None) => {
                    verdict(Tone::Danger, "Another file matches this episode", true)
                },
                (_, Some(Conflict::Existing), None) => verdict(Tone::Danger, "This episode already has a file", true),
                _ if target(row).is_some_and(|name| names[&name] > 1) => {
                    verdict(Tone::Danger, "Same name as another file", true)
                },
                (_, Some(_), Some(Resolution::Replace)) => verdict(Tone::Info, "Replaces the other file", false),
                (_, Some(_), Some(Resolution::KeepBoth)) => verdict(Tone::Info, "Kept alongside the other file", false),
                _ => match row.confidence {
                    Confidence::Certain => verdict(Tone::Success, "Certain match", false),
                    Confidence::Guess => verdict(Tone::Warning, "Guess, check it", false),
                    Confidence::Unknown => verdict(Tone::Warning, "Chosen by you", false),
                },
            };
            (row.id, verdict)
        })
        .collect()
}

fn parse_episode(value: &str) -> Result<Option<u32>> {
    if value.is_empty() {
        return Ok(None);
    }
    let episode = value.parse().ok().filter(|episode| EPISODES.iter().any(|(number, _)| number == episode));
    Ok(Some(episode.ok_or_else(|| bad_request("unknown episode"))?))
}

#[procedure]
async fn set_episode(cx: &Cx, row: u64, episode: String) -> Result<usize> {
    let episode = parse_episode(&episode)?;
    Ok(app_context::<Review>(cx).update(
        |candidate| candidate.id == row,
        |row| {
            row.episode = episode;
            row.resolution = None;
        },
    ))
}

#[procedure]
async fn set_resolution(cx: &Cx, row: u64, resolution: String) -> Result<usize> {
    let resolution = Resolution::parse(&resolution);
    Ok(app_context::<Review>(cx).update(|candidate| candidate.id == row, |row| row.resolution = resolution))
}

#[procedure]
async fn set_selected(cx: &Cx, row: u64, selected: bool) -> Result<usize> {
    Ok(app_context::<Review>(cx).update(|candidate| candidate.id == row, |row| row.selected = selected))
}

#[procedure]
async fn select_all(cx: &Cx, selected: bool) -> Result<usize> {
    Ok(app_context::<Review>(cx).update(|_| true, |row| row.selected = selected))
}

#[procedure]
async fn skip_selected(cx: &Cx) -> Result<usize> {
    Ok(app_context::<Review>(cx).update(|row| row.selected, |row| row.resolution = Some(Resolution::Skip)))
}

/// Gives the selected rows consecutive episodes, starting from the first selected row's episode.
#[procedure]
async fn assign_in_order(cx: &Cx) -> Result<usize> {
    let review = app_context::<Review>(cx);
    let first = review.rows().into_iter().find(|row| row.selected).and_then(|row| row.episode).unwrap_or(1);
    let mut next = first;
    Ok(review.update(
        |row| row.selected,
        |row| {
            row.episode = EPISODES.iter().any(|(number, _)| *number == next).then_some(next);
            row.resolution = None;
            next += 1;
        },
    ))
}

#[query_params(error = bad_request)]
struct ReviewQuery {
    imported: Option<usize>,
}

#[page("/patterns/import-review")]
pub(crate) async fn import_review_story(cx: &Cx) -> Result<impl View> {
    let imported = topcoat::router::query_params::<ReviewQuery>(cx)?.imported;
    Ok(view! {
        story_page(
            name: "Import review",
            path: "…",
            summary: "One row per file: file → episode → new name. Correct rows, resolve conflicts, then import.",
            if let Some(count) = imported {
                alert(
                    variant: AlertVariant::Info,
                    attrs: attributes! { role="status" },
                    alert_title((format!("Imported {count} files.")))
                )
            }
            story(title: "Orbital (2024) · Season 1 pack", review_table())
            <form method="post" action=(href!(reset_review).resolve(cx))>
                button(
                    variant: ButtonVariant::Quiet,
                    attrs: attributes! { type="submit" },
                    "Reset the demo"
                )
            </form>
        )
    })
}

/// The review rows, selection bar, and import action; every edit re-renders this shard.
#[shard]
async fn review_table(cx: &Cx) -> Result<impl View> {
    let version = signal(cx, || 0usize);
    let _ = version.get();
    let rows = app_context::<Review>(cx).rows();
    let verdicts = verdicts(&rows);
    let conflicts = conflicts(&rows);
    let total = rows.len();
    let selected = rows.iter().filter(|row| row.selected).count();
    let some_selected = selected > 0 && selected < total;
    let importing = rows.iter().filter_map(target).count();
    let blocking = verdicts.values().filter(|verdict| verdict.blocking).count();
    let summary = format!("{selected} of {total} files selected");

    Ok(view! {
        <div class="flex flex-col gap-4">
            <div class="flex items-center gap-2">
                checkbox(
                    attrs: attributes! {
                        id="select-all"
                        checked=(selected == total)
                        :indeterminate=$({
                            let _rendered = version.get();
                            some_selected
                        })
                        @change=$(async |e: Event| {
                            let _changed = select_all(e.target.checked).await;
                            version.increment()
                        })
                    }
                )
                <label for="select-all" class="text-caption text-muted">
                    "Select all"
                </label>
            </div>
            if selected > 0 {
                selection_bar(
                    summary: &summary,
                    button(
                        attrs: attributes! {
                            @click=$(async |_e: Event| {
                                let _changed = assign_in_order().await;
                                version.increment()
                            })
                        },
                        "Assign episodes in order"
                    )
                    button(
                        attrs: attributes! {
                            @click=$(async |_e: Event| {
                                let _changed = skip_selected().await;
                                version.increment()
                            })
                        },
                        "Skip selected"
                    )
                )
            }
            <div>
                #[key(row.id)]
                for row in rows {
                    let id = row.id;
                    let verdict = &verdicts[&id];
                    let conflict = conflicts.get(&id).copied();
                    rename_row(
                        old: row.file,
                        checkbox_attrs: attributes! {
                            checked=(row.selected)
                            @change=$(async |e: Event| {
                                let _changed = set_selected(id, e.target.checked).await;
                                version.increment()
                            })
                        },
                        <div class="grid gap-1.5">
                            select(
                                attrs: attributes! {
                                    aria-label=(format!("Episode for {}", row.file))
                                    aria-invalid=(verdict.blocking.then_some("true"))
                                    class="font-mono text-caption"
                                    @change=$(async |e: Event| {
                                        let _changed = set_episode(id, e.target.value).await;
                                        version.increment()
                                    })
                                },
                                <option value="" selected=(row.episode.is_none())>
                                    "Choose the correct name…"
                                </option>
                                for (number, title) in EPISODES {
                                    <option
                                        value=(number)
                                        selected=(row.episode == Some(number))
                                    >
                                        (format!("{SERIES} - S01E{number:02} - {title}.mkv"))
                                    </option>
                                }
                            )
                            <div class="flex flex-wrap items-center gap-x-3 gap-y-1.5">
                                status(tone: verdict.tone, label: &verdict.label)
                                if conflict.is_some() {
                                    select(
                                        attrs: attributes! {
                                            aria-label=(format!("Conflict for {}", row.file))
                                            class="w-auto text-caption"
                                            @change=$(async |e: Event| {
                                                let _changed = set_resolution(id, e.target.value).await;
                                                version.increment()
                                            })
                                        },
                                        <option value="" selected=(row.resolution.is_none())>
                                            "Resolve the conflict…"
                                        </option>
                                        for resolution in Resolution::ALL {
                                            <option
                                                value=(resolution.value())
                                                selected=(row.resolution == Some(resolution))
                                            >
                                                (resolution.label())
                                            </option>
                                        }
                                    )
                                }
                                if row.resolution == Some(Resolution::KeepBoth) {
                                    <span class="yk-code text-muted">
                                        (target(&row).unwrap_or_default())
                                    </span>
                                }
                            </div>
                        </div>
                    )
                }
            </div>
            <form
                method="post"
                action=(href!(import_files).resolve(cx))
                class="flex flex-wrap items-center justify-end gap-3 border-t border-line pt-4"
            >
                <span class="mr-auto text-caption text-muted" aria-live="polite">
                    (format!("{importing} files will be imported"))
                    if blocking > 0 {
                        (format!(" · {blocking} need attention"))
                    }
                </span>
                button(
                    variant: ButtonVariant::Primary,
                    attrs: attributes! { type="submit" disabled=(blocking > 0) },
                    (format!("Import {importing} files"))
                )
            </form>
        </div>
    })
}

/// Imports the plan when nothing blocks it.
#[route(POST "/patterns/import-review")]
async fn import_files(cx: &Cx) -> Result<SeeOther> {
    let rows = app_context::<Review>(cx).rows();
    if verdicts(&rows).values().any(|verdict| verdict.blocking) {
        return Err(bad_request("the import plan has unresolved rows").into());
    }
    let imported = rows.iter().filter_map(target).count();
    Ok(see_other(href!(import_review_story).query([("imported", imported)]).resolve(cx)))
}

#[route(POST "/patterns/import-review/reset")]
async fn reset_review(cx: &Cx) -> Result<SeeOther> {
    *app_context::<Review>(cx).rows.lock().unwrap() = Review::new().rows();
    Ok(see_other(href!(import_review_story).resolve(cx)))
}
