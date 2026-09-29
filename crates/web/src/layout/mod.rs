mod back_button;
pub mod document_head;

use dioxus::prelude::*;
use dioxus_icons::lucide::{CalendarDays, Library};

pub(crate) use self::back_button::BackButton;
use self::{back_button::use_in_app_history, document_head::DocumentHead};
use crate::{
    components::{
        sidebar::{
            Sidebar, SidebarContent, SidebarFooter, SidebarHeader, SidebarInset, SidebarMenu, SidebarMenuButton,
            SidebarMenuItem, SidebarProvider, SidebarTrigger, use_sidebar,
        },
        theme_switch::ThemeSwitch,
    },
    route::Route,
};

/// The sidebar with the main navigation, and the page beside it. Below `md` the sidebar is a
/// sheet opened from the top bar.
#[component]
pub fn Shell() -> Element {
    use_in_app_history();
    rsx! {
        document::Title { "Yokoku" }
        DocumentHead {}
        SidebarProvider {
            Sidebar {
                SidebarHeader { class: "px-4 py-3",
                    Brand {}
                }
                SidebarContent { class: "px-2",
                    nav { aria_label: "Main",
                        SidebarMenu {
                            NavItem { to: Route::Library {}, label: "Library", Library {} }
                            NavItem { to: Route::Upcoming {}, label: "Upcoming", CalendarDays {} }
                        }
                    }
                }
                SidebarFooter { class: "px-2 py-3",
                    ThemeSwitch {}
                }
            }
            SidebarInset { class: "overflow-y-auto",
                div { class: "flex items-center gap-2 border-b border-line px-3 py-2 md:hidden",
                    SidebarTrigger {}
                    Brand {}
                }
                div { class: "mx-auto flex w-full max-w-[80rem] flex-1 flex-col px-5 sm:px-8",
                    div { class: "flex-1 py-8", Outlet::<Route> {} }
                    Footer {}
                }
            }
        }
    }
}

#[component]
fn Brand() -> Element {
    rsx! {
        Link {
            to: Route::Library {},
            class: "font-mono text-xl tracking-tight",
            "yokoku"
        }
    }
}

/// A main destination, active on its own pages; following it closes the sidebar sheet.
#[component]
fn NavItem(to: Route, label: &'static str, children: Element) -> Element {
    let sidebar = use_sidebar();
    let active = use_route::<Route>().section() == to;
    rsx! {
        SidebarMenuItem {
            SidebarMenuButton {
                is_active: active,
                r#as: move |attributes: Vec<Attribute>| rsx! {
                    Link {
                        to: to.clone(),
                        onclick: move |_| sidebar.set_open_mobile(false),
                        attributes,
                        {children.clone()}
                        span { "{label}" }
                    }
                },
            }
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
