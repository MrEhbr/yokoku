mod torrents;

use std::time::Duration;

use dioxus::prelude::*;

use self::torrents::Torrents;
use crate::{
    api::downloads::{DownloadEntry, downloads, live_downloads},
    components::{
        alert::{Alert, AlertDescription, AlertTitle, AlertVariant},
        skeleton::Skeleton,
    },
};

const RECONNECT: Duration = Duration::from_secs(2);

/// Downloads as of the last sync with the download client, each with the state of its import
/// (FR-3.4, 4.11, 9.2). Rendered on the server, then updated in place from the changes it pushes,
/// reconnecting when the stream drops.
#[component]
pub fn Downloads() -> Element {
    let first = use_server_future(downloads)?;
    let mut latest = use_signal(|| None::<Vec<DownloadEntry>>);
    use_future(move || async move {
        if !cfg!(target_arch = "wasm32") {
            return;
        }
        loop {
            if let Ok(mut updates) = live_downloads().await {
                while let Some(Ok(current)) = updates.recv().await {
                    latest.set(Some(current));
                }
            }
            pause(RECONNECT).await;
        }
    });

    let first = first.read();
    let current = match (latest.read().clone(), &*first) {
        (Some(reloaded), _) => Some(Ok(reloaded)),
        (None, first) => first.clone(),
    };
    rsx! {
        document::Title { "Downloads · Yokoku" }
        h1 { class: "yk-page-title", "Downloads" }
        p { class: "mt-2 text-muted", "As of the last sync with the download client." }
        div { class: "mt-8",
            match current {
                None => rsx! {
                    Skeleton { class: "h-64 w-full" }
                },
                Some(Err(_)) => rsx! {
                    Alert { variant: AlertVariant::Danger,
                        AlertTitle { "Downloads could not be loaded" }
                        AlertDescription { "Reload the page; if it keeps failing, the server log has the cause." }
                    }
                },
                Some(Ok(downloads)) => rsx! {
                    Torrents { downloads }
                },
            }
        }
    }
}

async fn pause(duration: Duration) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
}
