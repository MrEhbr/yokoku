mod episodes;
mod summary;

use dioxus::prelude::*;
use yokoku_domain::{ItemId, SeriesId};

use self::{episodes::SeasonItem, summary::EpisodeSummary};
use crate::{
    api::library::detail::{self, Numbering, series},
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        history_list::{HistoryList, HistoryScope},
        item_description::ItemDescription,
        item_hero::ItemHero,
        item_status::{Lifecycle, Monitoring},
        skeleton::Skeleton,
    },
    format::year,
    layout::BackButton,
    route::Route,
};

/// A series with its next and last episodes, and every season's episodes with air date and
/// file status (FR-1.4, 6.1, 6.2).
#[component]
pub fn SeriesDetail(id: SeriesId) -> Element {
    let series = use_server_future(use_reactive!(|id| series(id)))?;

    rsx! {
        BackButton { fallback: Route::Library {} }
        div { class: "mt-4",
            match &*series.read() {
                None => rsx! {
                    Skeleton { class: "aspect-[3/1] w-full" }
                },
                Some(Err(_)) => rsx! {
                    Alert { variant: AlertVariant::Danger,
                        AlertTitle { "The series could not be loaded" }
                        AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
                    }
                },
                Some(Ok(None)) => rsx! {
                    document::Title { "Series not found · Yokoku" }
                    h1 { class: "yk-page-title", "Series not found" }
                    p { class: "mt-2 text-muted", "It may have been removed from the library." }
                },
                Some(Ok(Some(series))) => rsx! {
                    Page { series: series.clone() }
                },
            }
        }
    }
}

#[component]
fn Page(series: detail::SeriesDetail) -> Element {
    let images = series.images.clone();
    let today = series.today;
    let (regular, specials): (Vec<_>, Vec<_>) = series.seasons.iter().cloned().partition(|season| season.number != 0);
    rsx! {
        document::Title { "{series.title} · Yokoku" }
        ItemHero {
            title: series.title.clone(),
            poster: images.poster,
            backdrop: images.backdrop,
            logo: images.logo,
            p { class: "text-muted",
                "{year(series.year)} · Series · "
                a {
                    class: "yk-code underline-offset-4 hover:text-ink hover:underline",
                    href: "{series.source_url}",
                    target: "_blank",
                    rel: "noreferrer",
                    "{series.source}"
                }
            }
            if series.original_title != series.title {
                p { class: "text-muted", "Original title: {series.original_title}" }
            }
            div { class: "flex flex-wrap gap-x-4 gap-y-1",
                Lifecycle { status: series.status }
                Monitoring { monitored: series.monitored }
                if series.numbering == Numbering::Absolute {
                    span { class: "text-caption text-muted", "Absolute numbering" }
                }
            }
            ItemDescription { description: series.description.clone(), per_episode: true }
            div { class: "mt-2 grid gap-3 lg:grid-cols-2",
                EpisodeSummary {
                    label: "Next episode",
                    none: "No upcoming episode announced",
                    episode: series.next.clone(),
                    today,
                }
                EpisodeSummary {
                    label: "Last aired",
                    none: "Nothing has aired yet",
                    episode: series.last.clone(),
                    today,
                }
            }
        }
        if series.seasons.is_empty() {
            p { class: "mt-10 text-muted", "The metadata source lists no episodes yet." }
        } else {
            div { class: "mt-10 border-t border-line",
                for (index, season) in regular.into_iter().rev().chain(specials).enumerate() {
                    SeasonItem {
                        key: "{season.number}",
                        open: index == 0,
                        season,
                        series_monitored: series.monitored,
                        today,
                    }
                }
            }
        }
        section { class: "mt-12",
            h2 { class: "text-section font-medium", "History" }
            div { class: "mt-4",
                HistoryList {
                    key: "{series.id}",
                    scope: HistoryScope::Item(ItemId::Series(series.id)),
                    empty: "Nothing has happened to this series yet.",
                }
            }
        }
    }
}
