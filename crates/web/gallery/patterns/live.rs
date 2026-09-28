//! Server-pushed job progress over SSE, reconnecting after the stream drops.

use std::time::Duration;

use dioxus::prelude::*;
use yokoku_web::components::ui::{
    button::{Button, ButtonVariant},
    progress::Progress,
};

use crate::patterns::backend::{progress, start_job};

#[component]
pub fn LiveProgress() -> Element {
    let mut percent = use_signal(|| None::<u32>);
    let mut connected = use_signal(|| false);

    use_future(move || async move {
        loop {
            if let Ok(mut events) = progress().await {
                connected.set(true);
                while let Some(Ok(value)) = events.recv().await {
                    percent.set(value);
                }
            }
            connected.set(false);
            pause(Duration::from_secs(1)).await;
        }
    });

    let detail = percent().map_or_else(|| "Idle".to_owned(), |percent| format!("{percent}%"));
    rsx! {
        h1 { class: "yk-page-title", "Live progress" }
        p { class: "mt-2 max-w-prose text-muted",
            "Server-sent events from a job on the server. Restart the server: the page reconnects on its own."
        }
        div { class: "mt-6 flex max-w-md flex-col gap-3 border border-line p-4",
            div { class: "flex justify-between text-caption",
                span { class: "font-medium", "Import · Orbital Season 1" }
                span { class: "text-muted tabular-nums", "{detail}" }
            }
            Progress {
                value: percent().map(f64::from),
                aria_label: "Import progress",
            }
            div { class: "flex items-center justify-between gap-3",
                span { class: "text-caption text-muted",
                    if connected() {
                        "Connected"
                    } else {
                        "Reconnecting…"
                    }
                }
                Button {
                    variant: ButtonVariant::Primary,
                    onclick: move |_| async move {
                        let _ = start_job().await;
                    },
                    "Start import"
                }
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
