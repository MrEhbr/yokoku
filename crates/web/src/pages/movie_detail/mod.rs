use dioxus::prelude::*;
use yokoku_domain::{ItemId, MovieId};

use crate::{
    api::library::detail::{self, movie},
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        file_info::FileDetails,
        history_list::{HistoryList, HistoryScope},
        item_description::ItemDescription,
        item_hero::ItemHero,
        item_status::{FileState, Lifecycle, Monitoring},
        skeleton::Skeleton,
    },
    format::{date, relative, year},
    layout::BackButton,
    route::Route,
};

/// A movie with its cinema, digital and physical release dates and file status (FR-1.5).
#[component]
pub fn MovieDetail(id: MovieId) -> Element {
    let movie = use_server_future(use_reactive!(|id| movie(id)))?;

    rsx! {
        BackButton { fallback: Route::Library {} }
        div { class: "mt-4",
            match &*movie.read() {
                None => rsx! {
                    Skeleton { class: "aspect-[3/1] w-full" }
                },
                Some(Err(_)) => rsx! {
                    Alert { variant: AlertVariant::Danger,
                        AlertTitle { "The movie could not be loaded" }
                        AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
                    }
                },
                Some(Ok(None)) => rsx! {
                    document::Title { "Movie not found · Yokoku" }
                    h1 { class: "yk-page-title", "Movie not found" }
                    p { class: "mt-2 text-muted", "It may have been removed from the library." }
                },
                Some(Ok(Some(movie))) => rsx! {
                    Page { movie: movie.clone() }
                },
            }
        }
    }
}

/// Each release shows how far away it is, which explains the lifecycle and file status: a
/// movie counts as released, and its file as missing, from its digital or physical release.
#[component]
fn Page(movie: detail::MovieDetail) -> Element {
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
            div { class: "flex flex-wrap gap-x-4 gap-y-1",
                Lifecycle { status: movie.status }
                Monitoring { monitored: movie.monitored }
                FileState { status: movie.file }
            }
            ItemDescription { description: movie.description.clone() }
            div { class: "mt-2 grid gap-6 lg:grid-cols-2",
                section {
                    h2 { class: "text-caption font-medium text-muted", "Releases" }
                    dl { class: "mt-1 grid max-w-md grid-cols-[auto_1fr_auto] gap-x-6",
                        for (release, day) in movie.releases {
                            div { key: "{release.label()}", class: "contents",
                                dt { class: "border-b border-line py-2 text-muted", "{release.label()}" }
                                dd { class: "border-b border-line py-2 tabular-nums",
                                    {day.map(date).unwrap_or_else(|| "Not announced".to_owned())}
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
                        div { class: "mt-2",
                            FileDetails { info }
                        }
                    }
                }
            }
        }
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
