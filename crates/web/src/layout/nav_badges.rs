use dioxus::prelude::*;
use dioxus_icons::lucide::TriangleAlert;

use super::live_downloads::LiveDownloads;
use crate::{
    api::{
        downloads::{DownloadState, ImportState},
        library::calendar::missing_count,
    },
    components::badge::{Badge, BadgeVariant},
    route::Route,
};

/// The missing episodes and movies, read again on each navigation and each finished import.
#[component]
pub(super) fn WantedBadge() -> Element {
    let route = use_route::<Route>();
    let LiveDownloads(downloads) = use_context();
    let imported =
        use_memo(move || downloads.read().as_ref().map(|all| all.iter().filter(|download| download.imported).count()));
    let count = use_resource(use_reactive!(|route| {
        let _ = (route, imported());
        async move { missing_count().await.ok() }
    }));
    let count = count().flatten().unwrap_or(0);
    rsx! {
        if count > 0 {
            Count { count }
            span { class: "sr-only", ", {count} missing" }
        }
    }
}

/// The downloads in progress, and those that need review or failed.
#[component]
pub(super) fn QueueBadge() -> Element {
    let LiveDownloads(downloads) = use_context();
    let downloads = downloads.read();
    let all = downloads.as_deref().unwrap_or_default();
    let active = all
        .iter()
        .filter(|download| {
            matches!(download.state, DownloadState::Queued | DownloadState::Checking | DownloadState::Downloading)
        })
        .count();
    let attention = all
        .iter()
        .filter(|download| {
            matches!(download.state, DownloadState::Error(_))
                || download
                    .import
                    .as_ref()
                    .is_some_and(|import| matches!(import.state, ImportState::NeedsReview | ImportState::Failed(_)))
        })
        .count();
    rsx! {
        if active > 0 {
            Count { count: active }
            span { class: "sr-only", ", {active} active" }
        }
        if attention > 0 {
            Badge { variant: BadgeVariant::Warning, aria_hidden: true,
                TriangleAlert {}
                "{attention}"
            }
            span { class: "sr-only", ", {attention} need attention" }
        }
    }
}

#[component]
fn Count(count: usize) -> Element {
    rsx! {
        Badge { aria_hidden: true,
            if count > 99 {
                "99+"
            } else {
                "{count}"
            }
        }
    }
}
