mod torrents;

use dioxus::prelude::*;

use self::torrents::Torrents;
use crate::{
    api::downloads::downloads,
    components::{load_failed::LoadFailed, skeleton::Skeleton},
    dialogs::add_torrent::AddTorrentButton,
    layout::LiveDownloads,
};

/// Downloads as of the last sync with the download client, each with the state of its import
/// (FR-3.4, 4.11, 9.2). Rendered on the server, then updated in place from [`LiveDownloads`].
#[component]
pub fn Downloads() -> Element {
    let first = use_server_future(downloads)?;
    let LiveDownloads(latest) = use_context();

    let first = first.read();
    let current = match (latest(), &*first) {
        (Some(reloaded), _) => Some(Ok(reloaded)),
        (None, first) => first.clone(),
    };
    rsx! {
        document::Title { "Queue · Yokoku" }
        div { class: "flex flex-wrap items-center gap-4",
            h1 { class: "yk-page-title", "Queue" }
            div { class: "ml-auto", AddTorrentButton {} }
        }
        p { class: "mt-2 text-muted", "As of the last sync with the download client." }
        div { class: "mt-8",
            match current {
                None => rsx! {
                    Skeleton { class: "h-64 w-full" }
                },
                Some(Err(_)) => rsx! {
                    LoadFailed { subject: "Downloads" }
                },
                Some(Ok(downloads)) => rsx! {
                    Torrents { downloads }
                },
            }
        }
    }
}
