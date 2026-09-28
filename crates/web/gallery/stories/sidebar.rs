use dioxus::prelude::*;
use dioxus_icons::lucide::{Calendar, Download, Library, Settings};
use yokoku_web::components::sidebar::{
    Sidebar, SidebarContent, SidebarGroup, SidebarGroupContent, SidebarGroupLabel, SidebarHeader, SidebarInset,
    SidebarMenu, SidebarMenuBadge, SidebarMenuButton, SidebarMenuItem, SidebarProvider, SidebarTrigger,
};

use crate::{Story, StoryPage};

#[component]
pub fn SidebarStory() -> Element {
    rsx! {
        StoryPage {
            name: "Sidebar",
            path: "sidebar",
            summary: "A desktop navigation panel that becomes a sliding sheet below md; bounded to the canvas here.",
            Story { title: "Navigation",
                SidebarProvider { class: "!h-[28rem] !min-h-0 transform-gpu overflow-hidden border border-line",
                    Sidebar {
                        SidebarHeader {
                            span { class: "px-2 font-mono text-xl tracking-tight", "yokoku" }
                        }
                        SidebarContent {
                            SidebarGroup {
                                SidebarGroupLabel { "Media" }
                                SidebarGroupContent {
                                    SidebarMenu {
                                        SidebarMenuItem {
                                            SidebarMenuButton { is_active: true,
                                                Library {}
                                                span { "Library" }
                                            }
                                        }
                                        SidebarMenuItem {
                                            SidebarMenuButton {
                                                Calendar {}
                                                span { "Upcoming" }
                                            }
                                        }
                                        SidebarMenuItem {
                                            SidebarMenuButton {
                                                Download {}
                                                span { "Downloads" }
                                            }
                                            SidebarMenuBadge { "3" }
                                        }
                                    }
                                }
                            }
                            SidebarGroup {
                                SidebarGroupLabel { "System" }
                                SidebarGroupContent {
                                    SidebarMenu {
                                        SidebarMenuItem {
                                            SidebarMenuButton {
                                                Settings {}
                                                span { "Settings" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    SidebarInset {
                        div { class: "flex h-14 shrink-0 items-center gap-2 border-b border-line px-4",
                            SidebarTrigger {}
                        }
                        div { class: "p-8",
                            h1 { class: "yk-page-title", "Library" }
                            p { class: "mt-2 text-muted", "Page content next to the sidebar." }
                        }
                    }
                }
            }
        }
    }
}
