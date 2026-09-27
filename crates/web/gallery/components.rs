//! Stories for Yokoku's own components.

use topcoat::{
    Result,
    router::page,
    view::{View, attributes, view},
};
use yokoku_web::components::{
    attribution::*,
    empty_state::*,
    job_progress::*,
    media_card::*,
    page_header::*,
    rename_row::*,
    selection_bar::*,
    status::*,
    ui::{alert::*, button::*, label::*, select::*},
};

use crate::{story, story_page};

const POSTER: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2 3'%3E%3Cdefs%3E%3ClinearGradient id='g' x2='0' y2='1'%3E%3Cstop stop-color='%23355e82'/%3E%3Cstop offset='1' stop-color='%23e99db4'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='2' height='3' fill='url(%23g)'/%3E%3C/svg%3E";

#[page("/components/attribution")]
pub(crate) async fn attribution_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Attribution", path: "attribution", summary: "The TMDB and TheTVDB notice required on pages that show metadata.",
            story(title: "Footer", attribution())
        )
    })
}

#[page("/components/empty-state")]
pub(crate) async fn empty_state_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Empty state", path: "empty_state", summary: "What to do when a view has nothing to show. Empty, loading, offline, and failed are different states.",
            story(title: "With description and action",
                empty_state(
                    title: "Your library is empty.",
                    description: "Add a movie or series to start tracking releases.",
                    button(variant: ButtonVariant::Primary, "Add a movie or series")
                )
            )
            story(title: "Empty filter", empty_state(title: "No results match these filters.", button("Clear filters")))
            story(title: "Title only", empty_state(title: "No files need review."))
        )
    })
}

#[page("/components/job-progress")]
pub(crate) async fn job_progress_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Job progress", path: "job_progress", summary: "A long operation with its numbers. Unknown progress is indeterminate; failures show the reason and a retry.",
            story(title: "Known progress",
                <div class="max-w-xl">job_progress(title: "Orbital · Season 1", detail: "64% · 4.1 MB/s · 2 min left", value: 64.0)</div>
            )
            story(title: "Unknown progress",
                <div class="max-w-xl">job_progress(title: "Scanning library", detail: "1,204 files checked")</div>
            )
            story(title: "Failed",
                <div class="max-w-xl">
                    job_progress(
                        title: "Night Train (2026)",
                        detail: "Stopped at 12%",
                        value: 12.0,
                        <div class="flex flex-wrap items-center justify-between gap-2">
                            status(tone: Tone::Danger, label: "Tracker unreachable")
                            button(size: ButtonSize::Sm, "Retry")
                        </div>
                    )
                </div>
            )
        )
    })
}

#[page("/components/media-card")]
pub(crate) async fn media_card_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Media card", path: "media_card", summary: "A movie or series: poster, linked title, and one line per status.",
            story(title: "Poster, placeholder, long title",
                <div class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-x-5 gap-y-8">
                    media_card(
                        title: "The Expanse", href: "#", meta: "2015 · Series", poster: POSTER,
                        status(tone: Tone::Muted, label: "Ended")
                        status(tone: Tone::Success, label: "All files present")
                    )
                    media_card(
                        title: "Orbital", href: "#", meta: "2024 · Series",
                        status(tone: Tone::Info, label: "Continuing")
                        status(tone: Tone::Warning, label: "2 missing")
                        <span class="font-mono text-caption text-muted">"Next: S02E04 · Oct 4"</span>
                    )
                    media_card(
                        title: "Night Train to a Very Long Title That Wraps", href: "#", meta: "2026 · Movie",
                        status(tone: Tone::Info, label: "In cinemas")
                        <span class="font-mono text-caption text-muted">"Digital: Nov 12"</span>
                    )
                </div>
            )
        )
    })
}

#[page("/components/page-header")]
pub(crate) async fn page_header_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Page header", path: "page_header", summary: "A page's title with its page-level actions.",
            story(title: "With actions",
                page_header(
                    title: "Library",
                    button("Rename files")
                    button(variant: ButtonVariant::Primary, "Add a movie or series")
                )
            )
            story(title: "Title only", page_header(title: "Settings"))
        )
    })
}

#[page("/components/rename-row")]
pub(crate) async fn rename_row_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Rename row", path: "rename_row", summary: "One file's old name, an arrow, and its new name or the control that chooses it.",
            story(title: "Rename preview",
                <div>
                    rename_row(
                        old: "The.Expanse.S01E01.1080p.WEB.mkv",
                        checkbox_attrs: attributes! { name="file" value="1" checked="" },
                        <span class="yk-code break-all">"The Expanse (2015) - S01E01 - Dulcinea.mkv"</span>
                    )
                    rename_row(
                        old: "The.Expanse.S01E03.1080p.WEB.en.srt",
                        checkbox_attrs: attributes! { name="file" value="3" },
                        <span class="yk-code break-all">"The Expanse (2015) - S01E03 - Remember the Cant.en.srt"</span>
                    )
                </div>
            )
            story(title: "Import review, with an unresolved match",
                <div class="flex flex-col gap-3">
                    <div>
                        rename_row(
                            old: "orbital.ep1.mkv",
                            checkbox_attrs: attributes! { name="file" value="1" checked="" },
                            select(
                                attrs: attributes! { aria-label="New name for orbital.ep1.mkv" class="font-mono text-caption" },
                                <option selected="">"Orbital (2024) - S01E01 - Launch.mkv"</option>
                                <option>"Orbital (2024) - S01E02 - Drift.mkv"</option>
                            )
                        )
                        rename_row(
                            old: "orbital.bonus.mkv",
                            checkbox_attrs: attributes! { name="file" value="2" checked="" },
                            select(
                                attrs: attributes! { aria-label="New name for orbital.bonus.mkv" aria-invalid="true" class="font-mono text-caption" },
                                <option value="" selected="">"Choose the correct name…"</option>
                                <option>"Orbital (2024) - S00E01 - Behind the Launch.mkv"</option>
                            )
                        )
                    </div>
                    alert(variant: AlertVariant::Warning, alert_title("1 file needs an episode match before import."))
                </div>
            )
        )
    })
}

#[page("/components/selection-bar")]
pub(crate) async fn selection_bar_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Selection bar", path: "selection_bar", summary: "The toolbar shown once rows are selected: the count, then actions valid for the selection.",
            story(title: "Summary only", selection_bar(summary: "2 of 3 files selected"))
            story(title: "With bulk tools and a mixed value",
                selection_bar(
                    summary: "2 files selected",
                    <div class="grid gap-1.5">
                        label(attrs: attributes! { for="bulk-season" class="text-caption" }, "Season")
                        select(attrs: attributes! { id="bulk-season" class="w-auto" }, <option>"Mixed"</option> <option>"Season 1"</option> <option>"Season 2"</option>)
                    </div>
                    button("Detect again")
                    button("Assign episodes in order")
                )
            )
        )
    })
}

#[page("/components/status")]
pub(crate) async fn status_story() -> Result<impl View> {
    Ok(view! {
        story_page(name: "Status", path: "status", summary: "A status as symbol, label, and color. Keep each category separate.",
            story(title: "Tones",
                <div class="flex flex-wrap gap-x-6 gap-y-3">
                    status(tone: Tone::Success, label: "Downloaded")
                    status(tone: Tone::Warning, label: "Missing")
                    status(tone: Tone::Danger, label: "Import failed")
                    status(tone: Tone::Info, label: "Not yet aired")
                    status(tone: Tone::Muted, label: "Not monitored")
                </div>
            )
            story(title: "Detection confidence",
                <div class="flex flex-wrap gap-x-6 gap-y-3">
                    status(tone: Tone::Success, label: "Certain")
                    status(tone: Tone::Warning, label: "Guess")
                    status(tone: Tone::Warning, label: "Unknown")
                </div>
            )
        )
    })
}
