use dioxus::prelude::*;
use dioxus_icons::lucide::Bookmark;

use crate::{
    api::{
        failure,
        library::manage::{MonitorTarget, set_monitored},
    },
    components::{
        button::{Button, ButtonSize, ButtonVariant},
        label::Label,
        switch::Switch,
    },
};

/// Turns monitoring of `target` on or off: a labelled switch for a whole item, or a bookmark,
/// filled while monitored, for a season or episode. It shows the new state at once and calls
/// `on_change` once the server has it; `name` says what it monitors, like `S01E02`.
#[component]
pub fn MonitorToggle(
    target: MonitorTarget,
    monitored: bool,
    name: String,
    #[props(default)] labelled: bool,
    on_change: Callback,
) -> Element {
    let mut wanted = use_signal(|| None::<bool>);
    let mut failed = use_signal(|| None::<String>);
    use_effect(use_reactive!(|monitored| {
        _ = monitored;
        wanted.set(None);
    }));
    let mut saving = use_signal(|| false);
    let shown = wanted().unwrap_or(monitored);
    let change = move |monitored: bool| async move {
        if saving() {
            return;
        }
        saving.set(true);
        wanted.set(Some(monitored));
        failed.set(None);
        match set_monitored(target, monitored).await {
            Ok(()) => on_change(()),
            Err(error) => {
                wanted.set(None);
                failed.set(Some(failure(&error)));
            },
        }
        saving.set(false);
    };
    rsx! {
        span { class: "inline-flex items-center gap-2",
            if labelled {
                Switch {
                    id: "monitored",
                    checked: Some(shown),
                    aria_busy: saving(),
                    on_checked_change: move |monitored| {
                        spawn(change(monitored));
                    },
                }
                Label { html_for: "monitored", "Monitored" }
            } else {
                Button {
                    variant: ButtonVariant::Quiet,
                    size: ButtonSize::Icon,
                    aria_pressed: shown,
                    aria_busy: saving(),
                    aria_label: "Monitor {name}",
                    title: if shown { "Monitored" } else { "Not monitored" },
                    onclick: move |_| change(!shown),
                    Bookmark {
                        size: "1rem",
                        class: if shown { "fill-current text-ink" } else { "text-muted" },
                    }
                }
            }
            if let Some(error) = failed() {
                span { role: "alert", class: "text-caption text-danger", "{error}" }
            }
        }
    }
}
