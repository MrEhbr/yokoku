pub mod add_torrent;
pub mod import_review;
mod pickers;
pub mod remove;
pub mod rename;

use dioxus::prelude::*;
use dioxus_icons::lucide::X;

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    dialog::{Dialog, DialogTitle},
};

/// A dialog titled `title` with a close button; `open` shows it. A `wide` one fits a table.
#[component]
fn ClosableDialog(title: &'static str, open: Signal<bool>, #[props(default)] wide: bool, children: Element) -> Element {
    rsx! {
        Dialog {
            open: Some(open()),
            on_open_change: move |next| open.set(next),
            class: if wide { "max-w-dialog-wide!" } else { "" },
            div { class: "flex items-start justify-between gap-4",
                DialogTitle { "{title}" }
                Button {
                    variant: ButtonVariant::Quiet,
                    size: ButtonSize::Icon,
                    aria_label: "Close",
                    onclick: move |_| open.set(false),
                    X {}
                }
            }
            {children}
        }
    }
}
