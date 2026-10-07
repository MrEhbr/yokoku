use dioxus::{core::Task, logger::tracing::warn, prelude::*};
use yokoku_domain::{ItemId, MovieId};

use crate::{
    api::{
        downloads::{DownloadState, ItemLink, item_downloads},
        library::{
            FileStatus,
            detail::{self, movie},
            manage::MonitorTarget,
        },
    },
    components::{
        button::ButtonSize,
        file_info::FileDetails,
        history_list::{HistoryList, HistoryScope},
        item_description::ItemDescription,
        item_hero::ItemHero,
        item_ratings::ItemRatings,
        item_status::{FileState, FileWatched, Lifecycle},
        load_failed::LoadFailed,
        monitor_toggle::MonitorToggle,
        skeleton::{Loaded, Skeleton},
        status::{Status, Tone},
        unrecognised_files::UnrecognisedFiles,
    },
    dialogs::{import_review::ReviewButton, item_actions::ItemActions},
    format::{date, relative, size, year},
    layout::BackButton,
    route::Route,
};

/// A movie with its cinema, digital and physical release dates and file status.
#[component]
pub fn MovieDetail(id: MovieId) -> Element {
    let movie = use_server_future(use_reactive!(|id| movie(id)))?;

    rsx! {
        BackButton { fallback: Route::Library {} }
        div { class: "mt-4",
            match &*movie.read() {
                None => rsx! {
                    Skeleton { class: "aspect-[3/1] max-h-[40dvh] w-full" }
                },
                Some(Err(_)) => rsx! {
                    LoadFailed { subject: "The movie" }
                },
                Some(Ok(None)) => rsx! {
                    document::Title { "Movie not found · Yokoku" }
                    h1 { class: "yk-page-title", "Movie not found" }
                    p { class: "mt-2 text-muted", "It may have been removed from the library." }
                },
                Some(Ok(Some(movie))) => rsx! {
                    Loaded {
                        Page { key: "{movie.id}", movie: movie.clone() }
                    }
                },
            }
        }
    }
}

/// Each release shows how far away it is. A movie counts as released, and its file as missing,
/// from its digital or physical release. The movie is read again after each change made on the page.
#[component]
fn Page(movie: detail::MovieDetail) -> Element {
    let id = movie.id;
    let mut current = use_signal(|| movie);
    let mut reading = use_signal(|| None::<Task>);
    let reload = use_callback(move |()| {
        if let Some(task) = reading.take() {
            task.cancel();
        }
        reading.set(Some(spawn(async move {
            match detail::movie(id).await {
                Ok(Some(movie)) => current.set(movie),
                Ok(None) => warn!("the movie is gone from the library"),
                Err(error) => warn!(%error, "reading the movie again failed"),
            }
        })));
    });
    let movie = current();
    let images = movie.images.clone();
    let today = movie.today;
    rsx! {
        document::Title { "{movie.title} · Yokoku" }
        ItemHero {
            title: movie.title.clone(),
            poster: images.poster,
            backdrop: images.backdrop,
            logo: images.logo,
            p { class: "text-muted",
                "{year(movie.year)} · Movie · "
                a {
                    class: "yk-code underline-offset-4 hover:text-ink hover:underline",
                    href: "{movie.source_url}",
                    target: "_blank",
                    rel: "noreferrer",
                    "{movie.source}"
                }
            }
            if movie.original_title != movie.title {
                p { class: "text-muted", "Original title: {movie.original_title}" }
            }
            ItemRatings { ratings: movie.ratings.clone() }
            div { class: "flex flex-wrap gap-x-4 gap-y-1",
                Lifecycle { status: movie.status }
                FileState { status: movie.file, monitored: movie.monitored }
                if let Some(info) = &movie.file_info {
                    span { class: "text-caption text-muted", "{size(info.size)} on disk" }
                }
                if movie.file == FileStatus::Downloaded {
                    FileWatched { watched: movie.watched }
                }
            }
            div { class: "flex flex-wrap items-center gap-x-6 gap-y-2",
                MonitorToggle {
                    target: MonitorTarget::Movie { id },
                    monitored: movie.monitored,
                    name: movie.title.clone(),
                    labelled: true,
                    on_change: reload,
                }
                ItemActions {
                    item: ItemLink {
                        id: ItemId::Movie(id),
                        title: movie.title.clone(),
                    },
                    on_change: reload,
                }
            }
            ItemDescription { description: movie.description.clone() }
            div { class: "mt-2 grid gap-6 lg:grid-cols-2",
                section {
                    h2 { class: "text-caption font-medium text-muted", "Releases" }
                    dl { class: "mt-1 grid max-w-md grid-cols-[auto_1fr_auto] gap-x-6",
                        for (release, day) in movie.releases {
                            div { key: "{release.label()}", class: "contents",
                                dt { class: "border-b border-line py-2 text-muted",
                                    "{release.label()}"
                                }
                                dd { class: "border-b border-line py-2 tabular-nums",
                                    {day.map_or_else(|| "Not announced".to_owned(), date)}
                                }
                                dd { class: "border-b border-line py-2 text-right text-caption text-muted",
                                    {day.map(|day| relative(day, today)).unwrap_or_default()}
                                }
                            }
                        }
                    }
                }
                if let Some(info) = movie.file_info {
                    section {
                        h2 { class: "text-caption font-medium text-muted", "File" }
                        div { class: "mt-2 flex flex-col items-start gap-3",
                            FileDetails { info }
                        }
                    }
                }
            }
        }
        if let Some(unrecognised) = movie.unrecognised {
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
        MovieTorrents { id: movie.id }
        section { class: "mt-12",
            h2 { class: "text-section font-medium", "History" }
            div { class: "mt-4",
                HistoryList {
                    key: "{movie.id}",
                    scope: HistoryScope::Item(ItemId::Movie(movie.id)),
                    empty: "Nothing has happened to this movie yet.",
                }
            }
        }
    }
}

/// Every torrent linked to this movie, including those since removed from Transmission.
#[component]
fn MovieTorrents(id: MovieId) -> Element {
    let listed = use_server_future(use_reactive!(|id| item_downloads(ItemId::Movie(id))))?;
    rsx! {
        section { class: "mt-12",
            div { class: "flex items-baseline justify-between gap-4",
                h2 { class: "text-section font-medium", "Torrents" }
                Link { to: Route::Downloads {}, class: "text-caption text-muted hover:text-ink hover:underline", "Open queue" }
            }
            p { class: "mt-1 text-caption text-muted", "Last known status from Transmission." }
            div { class: "mt-4",
                match &*listed.read() {
                    None => rsx! { Skeleton { class: "h-20 w-full" } },
                    Some(Err(_)) => rsx! { LoadFailed { subject: "The movie's torrents" } },
                    Some(Ok(downloads)) if downloads.is_empty() => rsx! {
                        p { class: "text-muted", "No torrents linked to this movie yet." }
                    },
                    Some(Ok(downloads)) => rsx! {
                        ul { class: "divide-y divide-line border-y border-line",
                            for download in downloads.clone() {
                                li { key: "{download.id}", class: "flex flex-wrap items-center justify-between gap-x-6 gap-y-2 py-3",
                                    span { class: "min-w-0 [overflow-wrap:anywhere]", "{download.name}" }
                                    div { class: "flex flex-wrap items-center gap-x-4 gap-y-1",
                                        Status {
                                            tone: match &download.state {
                                                DownloadState::Error(_) => Tone::Danger,
                                                DownloadState::Removed | DownloadState::Paused => Tone::Muted,
                                                DownloadState::Finished | DownloadState::Seeding => Tone::Success,
                                                _ => Tone::Info,
                                            },
                                            label: if matches!(&download.state, DownloadState::Downloading | DownloadState::Checking | DownloadState::Paused) {
                                                format!("{} · {}%", download.state.label(), download.percent)
                                            } else {
                                                download.state.label().to_owned()
                                            },
                                        }
                                        if download.imported {
                                            Status { tone: Tone::Success, label: "Imported".to_owned() }
                                        } else if download.percent == 100 {
                                            Status { tone: Tone::Warning, label: "Not imported".to_owned() }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}
