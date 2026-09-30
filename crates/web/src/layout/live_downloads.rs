use std::time::Duration;

use dioxus::prelude::*;

use crate::api::downloads::{DownloadEntry, live_downloads};

const RECONNECT: Duration = Duration::from_secs(2);

/// The downloads as the server last pushed them, or none before the first push; shared by the
/// top bar and the Queue page.
#[derive(Clone, Copy)]
pub(crate) struct LiveDownloads(pub(crate) ReadSignal<Option<Vec<DownloadEntry>>>);

/// Follows the downloads the server pushes, reconnecting when the stream drops, and provides
/// them as [`LiveDownloads`]; the shell calls it once.
pub(super) fn use_live_downloads() {
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
    use_context_provider(|| LiveDownloads(latest.into()));
}

async fn pause(duration: Duration) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(duration).await;
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
}
