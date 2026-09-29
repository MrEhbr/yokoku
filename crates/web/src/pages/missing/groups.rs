use dioxus::prelude::*;

use crate::{
    api::library::calendar::{Missing, MissingSeries},
    components::{
        disclosure::Disclosure,
        table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    },
    format::{date, episode as code, year},
    route::Route,
};

/// A count of everything missing, then one closed group per series and the movies.
#[component]
pub(super) fn Groups(missing: Missing) -> Element {
    let episodes: usize = missing.series.iter().map(|series| series.episodes.len()).sum();
    let parts: Vec<String> = [
        (episodes > 0).then(|| {
            format!("{} in {}", count(episodes, "episode", "episodes"), count(missing.series.len(), "series", "series"))
        }),
        (!missing.movies.is_empty()).then(|| count(missing.movies.len(), "movie", "movies")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let summary = parts.join(" and ");
    rsx! {
        p { class: "font-medium", "{summary}" }
        if !missing.series.is_empty() {
            div { class: "mt-6 border-t border-line",
                for series in missing.series {
                    SeriesGroup { key: "{series.id}", series }
                }
            }
        }
        if !missing.movies.is_empty() {
            section { class: "mt-10",
                h2 { class: "text-section font-medium", "Movies" }
                ul { class: "mt-3 border-t border-line",
                    for movie in missing.movies {
                        li { key: "{movie.id}", class: "border-b border-line py-3",
                            Link {
                                class: "font-medium hover:underline",
                                to: Route::MovieDetail { id: movie.id },
                                "{movie.title}"
                            }
                            span { class: "ml-2 text-muted", "{year(movie.year)}" }
                        }
                    }
                }
            }
        }
    }
}

/// A series' missing episodes, newest first.
#[component]
fn SeriesGroup(series: MissingSeries) -> Element {
    let mut episodes = series.episodes;
    episodes.reverse();
    let latest = episodes.first().map(|episode| format!("latest {}", code(episode.season, episode.number)));
    let count = count(episodes.len(), "episode", "episodes");
    rsx! {
        Disclosure {
            summary: rsx! {
                span { class: "flex flex-wrap items-baseline gap-x-4 gap-y-1",
                    span { class: "text-section font-medium", "{series.title}" }
                    span { class: "text-caption text-muted",
                        "{year(series.year)} · {count} missing"
                        if let Some(latest) = latest {
                            ", {latest}"
                        }
                    }
                }
            },
            Link {
                class: "text-caption text-muted underline-offset-4 hover:text-ink hover:underline",
                to: Route::SeriesDetail { id: series.id },
                "Open {series.title}"
            }
            Table { class: "mt-2 min-w-[32rem] table-fixed", aria_label: "Missing episodes of {series.title}",
                TableHeader {
                    TableRow {
                        TableHead { class: "w-28", "Episode" }
                        TableHead { "Title" }
                        TableHead { class: "w-44", "Aired" }
                    }
                }
                TableBody {
                    for episode in episodes {
                        TableRow { key: "{episode.season}-{episode.number}",
                            TableCell { class: "yk-code whitespace-nowrap",
                                "{code(episode.season, episode.number)}"
                            }
                            TableCell { "{episode.title}" }
                            TableCell { class: "tabular-nums whitespace-nowrap", "{date(episode.air_date)}" }
                        }
                    }
                }
            }
        }
    }
}

/// `1 episode`, `3 episodes`.
fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}
