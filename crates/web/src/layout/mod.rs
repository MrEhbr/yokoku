mod back_button;
pub mod document_head;
mod live_downloads;
mod nav_badges;

use dioxus::prelude::*;
use dioxus_icons::lucide::Settings;

pub(crate) use self::{back_button::BackButton, live_downloads::LiveDownloads};
use self::{
    back_button::use_in_app_history,
    document_head::DocumentHead,
    live_downloads::use_live_downloads,
    nav_badges::{QueueBadge, WantedBadge},
};
use crate::{components::theme_switch::ThemeSwitch, route::Route};

const TMDB_LOGO: Asset = asset!("/assets/tmdb.svg");

/// Page width and side padding, shared by the top bar and the page so their edges line up.
const CONTAINER: &str = "mx-auto w-full max-w-[120rem] px-4 sm:px-6 lg:px-10";

/// The top bar with the main navigation, and the page below it. Below `md` the navigation
/// wraps to its own row, which scrolls sideways when it doesn't fit.
#[component]
pub fn Shell() -> Element {
    use_in_app_history();
    use_live_downloads();
    let section = use_route::<Route>().section();
    rsx! {
        document::Title { "Yokoku" }
        DocumentHead {}
        div { class: "flex min-h-dvh flex-col",
            header { class: "sticky top-0 z-40 border-b border-line bg-canvas",
                div { class: "{CONTAINER} flex flex-wrap items-center gap-x-8",
                    Link {
                        to: Route::Library {},
                        class: "py-3 font-mono text-xl tracking-tight",
                        "yokoku"
                    }
                    nav {
                        aria_label: "Main",
                        class: "order-last flex w-full overflow-x-auto md:order-none md:w-auto",
                        NavItem { to: Route::Library {}, label: "Library" }
                        NavItem { to: Route::Missing {}, label: "Wanted", WantedBadge {} }
                        NavItem { to: Route::Upcoming {}, label: "Upcoming" }
                        NavItem { to: Route::Downloads {}, label: "Queue", QueueBadge {} }
                        NavItem { to: Route::History {}, label: "History" }
                    }
                    div { class: "ml-auto flex items-center gap-1",
                        Link {
                            to: Route::Settings {},
                            class: "inline-flex size-9 items-center justify-center text-muted transition-colors hover:bg-subtle \
                                    hover:text-ink aria-[current=page]:bg-subtle aria-[current=page]:text-ink [&>svg]:size-4",
                            aria_label: "Settings",
                            aria_current: if section == (Route::Settings {}) { "page" } else { "false" },
                            Settings {}
                        }
                        ThemeSwitch {}
                    }
                }
            }
            main { class: "{CONTAINER} flex flex-1 flex-col",
                div { class: "flex-1 py-8", Outlet::<Route> {} }
                Footer {}
            }
        }
    }
}

/// A main destination, marked current on its own pages, with its badges after the label.
#[component]
fn NavItem(to: Route, label: &'static str, children: Element) -> Element {
    let active = use_route::<Route>().section() == to;
    rsx! {
        Link {
            to: to.clone(),
            class: "-mb-px flex shrink-0 items-center gap-1.5 border-b-2 border-transparent px-3 py-3 text-body \
                    whitespace-nowrap text-muted transition-colors hover:text-ink aria-[current=page]:border-ink \
                    aria-[current=page]:font-medium aria-[current=page]:text-ink",
            aria_current: if active { "page" } else { "false" },
            "{label}"
            {children}
        }
    }
}

/// Version and the metadata attribution every page showing metadata carries (FR-10.5).
#[component]
fn Footer() -> Element {
    let version = env!("CARGO_PKG_VERSION");
    rsx! {
        footer { class: "flex flex-col gap-1 border-t border-line py-4 text-caption text-muted",
            p { "Yokoku {version}" }
            a {
                class: "my-1 w-fit",
                href: "https://www.themoviedb.org",
                target: "_blank",
                rel: "noreferrer",
                img {
                    class: "h-3 w-auto",
                    src: TMDB_LOGO,
                    alt: "The Movie Database (TMDB)",
                }
            }
            p {
                "This product uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB. Metadata provided by "
                a {
                    class: "underline hover:text-ink",
                    href: "https://thetvdb.com",
                    "TheTVDB"
                }
                "."
            }
        }
    }
}
