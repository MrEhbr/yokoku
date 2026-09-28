use dioxus::prelude::*;
use yokoku_web::components::ui::breadcrumb::{
    Breadcrumb, BreadcrumbEllipsis, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator,
};

use crate::{Story, StoryPage};

#[component]
pub fn BreadcrumbStory() -> Element {
    rsx! {
        StoryPage {
            name: "Breadcrumb",
            path: "ui::breadcrumb",
            summary: "The path from a top-level page to the current one.",
            Story { title: "With ellipsis",
                Breadcrumb {
                    BreadcrumbList {
                        BreadcrumbItem {
                            BreadcrumbLink { href: "#", "Library" }
                        }
                        BreadcrumbSeparator {}
                        BreadcrumbItem { BreadcrumbEllipsis {} }
                        BreadcrumbSeparator {}
                        BreadcrumbItem {
                            BreadcrumbPage { "Season 1" }
                        }
                    }
                }
            }
        }
    }
}
