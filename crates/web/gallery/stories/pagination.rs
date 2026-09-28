use dioxus::prelude::*;
use yokoku_web::components::pagination::{
    Pagination, PaginationContent, PaginationEllipsis, PaginationItem, PaginationLink, PaginationNext,
    PaginationPrevious,
};

use crate::{Story, StoryPage};

#[component]
pub fn PaginationStory() -> Element {
    rsx! {
        StoryPage {
            name: "Pagination",
            path: "pagination",
            summary: "Page links for a list too long to show on one screen.",
            Story { title: "Middle page",
                Pagination {
                    PaginationContent {
                        PaginationItem {
                            PaginationPrevious { href: "#" }
                        }
                        PaginationItem {
                            PaginationLink { href: "#", "1" }
                        }
                        PaginationItem {
                            PaginationLink { is_active: true, href: "#", "2" }
                        }
                        PaginationItem { PaginationEllipsis {} }
                        PaginationItem {
                            PaginationNext { href: "#" }
                        }
                    }
                }
            }
        }
    }
}
