mod groups;

use dioxus::prelude::*;

use self::groups::Groups;
use crate::{
    api::library::calendar::missing,
    components::{load_failed::LoadFailed, skeleton::Skeleton},
};

/// Monitored episodes that aired without a file, grouped by series, and released monitored
/// movies without one (FR-6.3, 6.4).
#[component]
pub fn Missing() -> Element {
    let missing = use_server_future(missing)?;

    rsx! {
        document::Title { "Wanted · Yokoku" }
        h1 { class: "yk-page-title", "Wanted" }
        p { class: "mt-2 text-muted",
            "Monitored episodes that have aired and released monitored movies, without a file."
        }
        div { class: "mt-8",
            match &*missing.read() {
                None => rsx! {
                    Skeleton { class: "h-64 w-full" }
                },
                Some(Err(_)) => rsx! {
                    LoadFailed { subject: "Missing files" }
                },
                Some(Ok(missing)) if missing.series.is_empty() && missing.movies.is_empty() => {
                    rsx! {
                        p { class: "text-muted", "Nothing is missing." }
                    }
                }
                Some(Ok(missing)) => rsx! {
                    Groups { missing: missing.clone() }
                },
            }
        }
    }
}
