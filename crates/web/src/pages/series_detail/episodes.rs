use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronRight;
use jiff::civil::Date;

use crate::{
    api::library::{FileStatus, detail::SeasonDetail},
    components::{
        disclosure::Disclosure,
        file_info::{FileDetails, FileSummary},
        item_status::{FileState, Monitoring},
        table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    },
    format::{date, episode as code, relative},
};

/// A season that opens to its episodes. Monitoring shows only where it is off, and only an
/// episode monitored with its season and series is missing.
#[component]
pub(super) fn SeasonItem(open: bool, season: SeasonDetail, series_monitored: bool, today: Date) -> Element {
    let name = season.name();
    let count = season.episodes.len();
    let followed = series_monitored && season.monitored;
    let downloaded = season.episodes.iter().filter(|episode| episode.file == FileStatus::Downloaded).count();
    let missing = season
        .episodes
        .iter()
        .filter(|episode| followed && episode.monitored && episode.file == FileStatus::Missing)
        .count();
    rsx! {
        Disclosure {
            open,
            summary: rsx! {
                span { class: "flex flex-wrap items-baseline gap-x-4 gap-y-1",
                    span { class: "text-section font-medium", "{name}" }
                    span { class: "text-caption text-muted",
                        "{downloaded} of {count} downloaded"
                        if missing > 0 {
                            ", {missing} missing"
                        }
                    }
                    if !season.monitored {
                        Monitoring { monitored: false }
                    }
                }
            },
            Table { class: "min-w-[36rem] table-fixed", aria_label: "{name} episodes",
                TableHeader {
                    TableRow {
                        TableHead { class: "w-28", "Episode" }
                        TableHead { "Title" }
                        TableHead { class: "w-44", "Air date" }
                        TableHead { class: "w-36", "File" }
                    }
                }
                TableBody {
                    for episode in season.episodes {
                        TableRow { key: "{episode.number}", class: "[&>td]:align-top",
                            TableCell { class: "yk-code whitespace-nowrap",
                                "{code(episode.season, episode.number)}"
                            }
                            TableCell {
                                if episode.overview.is_empty() && episode.file_info.is_none() {
                                    EpisodeTitle { title: episode.title.clone(), monitored: episode.monitored }
                                } else {
                                    details { class: "group/episode",
                                        summary { class: "flex cursor-pointer list-none items-baseline gap-2 hover:underline [&::-webkit-details-marker]:hidden",
                                            ChevronRight {
                                                size: "0.75rem",
                                                class: "shrink-0 self-center text-muted transition-transform group-open/episode:rotate-90 motion-reduce:transition-none",
                                            }
                                            EpisodeTitle { title: episode.title.clone(), monitored: episode.monitored }
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

/// The title, `TBA` while it has none, and whether the episode is unmonitored.
#[component]
fn EpisodeTitle(title: String, monitored: bool) -> Element {
    rsx! {
        if title.is_empty() {
            span { class: "text-muted", "TBA" }
        } else {
            span { "{title}" }
        }
        if !monitored {
            span { class: "ml-3",
                Monitoring { monitored: false }
            }
        }
    }
}
