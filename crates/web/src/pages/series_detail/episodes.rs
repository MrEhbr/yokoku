use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronRight;
use jiff::civil::Date;
use yokoku_domain::SeriesId;

use crate::{
    api::library::{
        FileStatus,
        detail::{EpisodeRow, SeasonDetail},
        manage::MonitorTarget,
    },
    components::{
        disclosure::Disclosure,
        file_info::{FileDetails, FileSummary},
        item_status::{FileState, FileWatched},
        monitor_toggle::MonitorToggle,
        table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    },
    format::{date, episode as code, relative},
};

/// A season that opens to its episodes, each season and episode with its monitoring toggle. Only
/// an episode monitored with its season and series is missing.
#[component]
pub(super) fn SeasonItem(
    open: bool,
    series: SeriesId,
    season: SeasonDetail,
    series_monitored: bool,
    today: Date,
    on_change: Callback,
) -> Element {
    let name = season.name();
    let count = season.episodes.len();
    let followed = series_monitored && season.monitored;
    let downloaded = season.episodes.iter().filter(|episode| episode.file == FileStatus::Downloaded).count();
    let watched = season.episodes.iter().filter(|episode| episode.watched.is_some()).count();
    let missing = season
        .episodes
        .iter()
        .filter(|episode| followed && episode.monitored && episode.file == FileStatus::Missing)
        .count();
    rsx! {
        Disclosure {
            open,
            lead: rsx! {
                MonitorToggle {
                    target: MonitorTarget::Season {
                        id: series,
                        season: season.number,
                    },
                    monitored: season.monitored,
                    name: name.clone(),
                    on_change,
                }
            },
            summary: rsx! {
                span { class: "flex flex-wrap items-baseline gap-x-4 gap-y-1",
                    span { class: "text-section font-medium", "{name}" }
                    span { class: "text-caption text-muted",
                        "{downloaded} of {count} downloaded"
                        if watched > 0 {
                            ", {watched} watched"
                        }
                        if missing > 0 {
                            ", {missing} missing"
                        }
                    }
                }
            },
            EpisodeTable {
                series,
                season,
                followed,
                today,
                on_change,
            }
        }
    }
}

/// A season's episodes, each with its monitoring toggle, air date and file.
#[component]
fn EpisodeTable(series: SeriesId, season: SeasonDetail, followed: bool, today: Date, on_change: Callback) -> Element {
    let name = season.name();
    rsx! {
        Table {
            class: "table-fixed sm:min-w-[39rem]",
            aria_label: "{name} episodes",
            TableHeader {
                TableRow {
                    TableHead { class: "w-12",
                        span { class: "sr-only", "Monitored" }
                    }
                    TableHead { class: "hidden w-28 sm:table-cell", "Episode" }
                    TableHead { "Title" }
                    TableHead { class: "hidden w-44 sm:table-cell", "Air date" }
                    TableHead { class: "hidden w-36 sm:table-cell", "File" }
                }
            }
            TableBody {
                for episode in season.episodes {
                    TableRow { key: "{episode.number}", class: "[&>td]:align-top",
                        TableCell {
                            div { class: "-my-2",
                                MonitorToggle {
                                    target: MonitorTarget::Episode {
                                        id: series,
                                        season: episode.season,
                                        episode: episode.number,
                                    },
                                    monitored: episode.monitored,
                                    name: code(episode.season, episode.number),
                                    on_change,
                                }
                            }
                        }
                        TableCell { class: "yk-code hidden whitespace-nowrap sm:table-cell",
                            "{code(episode.season, episode.number)}"
                        }
                        TableCell {
                            if episode.overview.is_empty() && episode.file_info.is_none() {
                                EpisodeTitle { title: episode.title.clone() }
                                EpisodeFacts { episode: episode.clone(), followed }
                            } else {
                                details { class: "group/episode",
                                    summary { class: "block cursor-pointer list-none [&::-webkit-details-marker]:hidden",
                                        span { class: "flex items-baseline gap-2 hover:underline",
                                            ChevronRight {
                                                size: "0.75rem",
                                                class: "shrink-0 self-center text-muted transition-transform group-open/episode:rotate-90 motion-reduce:transition-none",
                                            }
                                            EpisodeTitle { title: episode.title.clone() }
                                        }
                                        span { class: "block pl-5",
                                            EpisodeFacts { episode: episode.clone(), followed }
                                        }
                                    }
                                    div { class: "mt-2 flex flex-col gap-3 pb-1 pl-5",
                                        if !episode.overview.is_empty() {
                                            p { class: "max-w-prose text-muted", "{episode.overview}" }
                                        }
                                        if let Some(info) = episode.file_info.clone() {
                                            FileDetails { info }
                                        }
                                    }
                                }
                            }
                        }
                        TableCell { class: "hidden tabular-nums whitespace-nowrap sm:table-cell",
                            match episode.air_date {
                                Some(aired) if aired >= today => rsx! {
                                    "{date(aired)}"
                                    span { class: "block text-caption text-muted", "{relative(aired, today)}" }
                                },
                                Some(aired) => rsx! { "{date(aired)}" },
                                None => rsx! { "—" },
                            }
                        }
                        TableCell { class: "hidden sm:table-cell",
                            FileState {
                                status: episode.file,
                                episode: true,
                                monitored: followed && episode.monitored,
                            }
                            if episode.file == FileStatus::Downloaded {
                                div {
                                    FileWatched { watched: episode.watched }
                                }
                            }
                            if let Some(info) = episode.file_info {
                                div {
                                    FileSummary { info }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Code, air date and file state on one line, in place of the columns hidden below `sm`.
#[component]
fn EpisodeFacts(episode: EpisodeRow, followed: bool) -> Element {
    rsx! {
        span { class: "mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-caption text-muted sm:hidden",
            span { class: "yk-code", "{code(episode.season, episode.number)}" }
            if let Some(aired) = episode.air_date {
                span { class: "tabular-nums", "{date(aired)}" }
            }
            FileState {
                status: episode.file,
                episode: true,
                monitored: followed && episode.monitored,
            }
            if episode.file == FileStatus::Downloaded {
                FileWatched { watched: episode.watched }
            }
        }
    }
}

/// The title, `TBA` while it has none.
#[component]
fn EpisodeTitle(title: String) -> Element {
    rsx! {
        if title.is_empty() {
            span { class: "text-muted", "TBA" }
        } else {
            span { "{title}" }
        }
    }
}
