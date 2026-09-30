use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronRight;
use jiff::civil::Date;
use yokoku_domain::SeriesId;

use crate::{
    api::library::{
        FileStatus,
        detail::SeasonDetail,
        manage::{FileOf, MonitorTarget},
    },
    components::{
        delete_file::DeleteFile,
        disclosure::Disclosure,
        file_info::{FileDetails, FileSummary},
        item_status::FileState,
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
    let missing = season
        .episodes
        .iter()
        .filter(|episode| followed && episode.monitored && episode.file == FileStatus::Missing)
        .count();
    let files: Vec<(String, u16, u16)> = season
        .episodes
        .iter()
        .filter_map(|episode| Some((episode.file_info.as_ref()?.path.clone(), episode.season, episode.number)))
        .collect();
    let sharing = move |path: &str, own: u16| -> Vec<String> {
        files
            .iter()
            .filter(|(other, _, number)| other == path && *number != own)
            .map(|(_, season, number)| code(*season, *number))
            .collect()
    };
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
                        if missing > 0 {
                            ", {missing} missing"
                        }
                    }
                }
            },
            Table {
                class: "min-w-[39rem] table-fixed",
                aria_label: "{name} episodes",
                TableHeader {
                    TableRow {
                        TableHead { class: "w-12",
                            span { class: "sr-only", "Monitored" }
                        }
                        TableHead { class: "w-28", "Episode" }
                        TableHead { "Title" }
                        TableHead { class: "w-44", "Air date" }
                        TableHead { class: "w-36", "File" }
                    }
                }
                TableBody {
                    for episode in season.episodes {
                        TableRow {
                            key: "{episode.number}",
                            class: "[&>td]:align-top",
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
                            TableCell { class: "yk-code whitespace-nowrap",
                                "{code(episode.season, episode.number)}"
                            }
                            TableCell {
                                if episode.overview.is_empty() && episode.file_info.is_none() {
                                    EpisodeTitle { title: episode.title.clone() }
                                } else {
                                    details { class: "group/episode",
                                        summary { class: "flex cursor-pointer list-none items-baseline gap-2 hover:underline [&::-webkit-details-marker]:hidden",
                                            ChevronRight {
                                                size: "0.75rem",
                                                class: "shrink-0 self-center text-muted transition-transform group-open/episode:rotate-90 motion-reduce:transition-none",
                                            }
                                            EpisodeTitle { title: episode.title.clone() }
                                        }
                                        div { class: "mt-2 flex flex-col gap-3 pb-1 pl-5",
                                            if !episode.overview.is_empty() {
                                                p { class: "max-w-prose text-muted",
                                                    "{episode.overview}"
                                                }
                                            }
                                            if let Some(info) = episode.file_info.clone() {
                                                FileDetails { info: info.clone() }
                                                div {
                                                    DeleteFile {
                                                        target: FileOf::Episode {
                                                            id: series,
                                                            season: episode.season,
                                                            episode: episode.number,
                                                        },
                                                        path: info.path.clone(),
                                                        also: sharing(&info.path, episode.number),
                                                        on_change,
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            TableCell { class: "tabular-nums whitespace-nowrap",
                                match episode.air_date {
                                    Some(aired) if aired >= today => rsx! {
                                        "{date(aired)}"
                                        span { class: "block text-caption text-muted", "{relative(aired, today)}" }
                                    },
                                    Some(aired) => rsx! { "{date(aired)}" },
                                    None => rsx! { "—" },
                                }
                            }
                            TableCell {
                                FileState {
                                    status: episode.file,
                                    episode: true,
                                    monitored: followed && episode.monitored,
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
