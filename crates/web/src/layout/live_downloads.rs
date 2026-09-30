use dioxus::prelude::*;

use crate::api::downloads::DownloadEntry;

/// The downloads as the server last pushed them, or none before the first push; shared by the
/// top bar and the Queue page.
#[derive(Clone, Copy)]
pub(crate) struct LiveDownloads(pub(crate) ReadSignal<Option<Vec<DownloadEntry>>>);

/// Follows the downloads the server pushes and provides them as [`LiveDownloads`]; the shell
/// calls it once.
pub(super) fn use_live_downloads() {
    let latest = use_signal(|| None::<Vec<DownloadEntry>>);
    #[cfg(target_arch = "wasm32")]
    use_future(move || follow(latest));
    use_context_provider(|| LiveDownloads(latest.into()));
}

/// Writes each push to `latest`, reconnecting when the stream drops.
#[cfg(target_arch = "wasm32")]
async fn follow(mut latest: Signal<Option<Vec<DownloadEntry>>>) {
    use std::time::Duration;

    use crate::api::downloads::live_downloads;

    const RECONNECT: Duration = Duration::from_secs(2);
    loop {
        if let Ok(mut updates) = live_downloads().await {
            while let Some(Ok(current)) = updates.recv().await {
                latest.set(Some(current));
            }
        }
        gloo_timers::future::sleep(RECONNECT).await;
    }
}
