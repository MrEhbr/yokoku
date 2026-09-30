mod episodes;
mod numbering;
mod summary;

use dioxus::{core::Task, logger::tracing::warn, prelude::*};
use yokoku_domain::{ItemId, SeriesId};

use self::{episodes::SeasonItem, numbering::NumberingSelect, summary::EpisodeSummary};
use crate::{
    api::library::{
        detail::{self, series},
        manage::MonitorTarget,
    },
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        button::ButtonSize,
        history_list::{HistoryList, HistoryScope},
        item_description::ItemDescription,
        item_hero::ItemHero,
        item_status::Lifecycle,
        monitor_toggle::MonitorToggle,
        refresh_button::RefreshButton,
        remove_item::RemoveItem,
        skeleton::Skeleton,
        unrecognised_files::UnrecognisedFiles,
    },
    dialogs::{add_torrent::AddTorrentButton, import_review::ReviewButton, rename::RenameButton},
    format::{size, year},
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
                    Page { key: "{series.id}", series: series.clone() }
                },
            }
        }
    }
}

/// The series as loaded, read again after each change made on the page.
#[component]
fn Page(series: detail::SeriesDetail) -> Element {
    let id = series.id;
    let mut current = use_signal(|| series);
    let mut reading = use_signal(|| None::<Task>);
    let reload = use_callback(move |()| {
        if let Some(task) = reading.take() {
            task.cancel();
        }
        reading.set(Some(spawn(async move {
            match detail::series(id).await {
                Ok(Some(series)) => current.set(series),
                Ok(None) => warn!("the series is gone from the library"),
                Err(error) => warn!(%error, "reading the series again failed"),
            }
        })));
    });
    let series = current();
    let usage = series.disk_usage();
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
                if usage.files > 0 {
                    span { class: "text-caption text-muted",
                        if usage.files == 1 {
                            "1 file"
                        } else {
                            "{usage.files} files"
                        }
                        " · {size(usage.size)} on disk"
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-x-6 gap-y-2",
                MonitorToggle {
                    target: MonitorTarget::Series { id },
                    monitored: series.monitored,
                    name: series.title.clone(),
                    labelled: true,
                    on_change: reload,
                }
                NumberingSelect { series: id, numbering: series.numbering, on_change: reload }
                div { class: "flex flex-wrap items-center gap-2",
                    AddTorrentButton { item: ItemId::Series(id) }
                    RefreshButton { item: ItemId::Series(id), on_change: reload }
                    RenameButton { item: ItemId::Series(id), on_change: reload }
                    RemoveItem { item: ItemId::Series(id), title: series.title.clone(), usage }
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
        if let Some(unrecognised) = series.unrecognised {
            div { class: "mt-8",
                UnrecognisedFiles { count: unrecognised.files,
                    ReviewButton {
                        import: unrecognised.import,
                        label: "Match",
                        size: ButtonSize::Sm,
                        on_done: reload,
                    }
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
                        series: id,
                        season,
                        series_monitored: series.monitored,
                        today,
                        on_change: reload,
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
