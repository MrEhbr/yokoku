use dioxus::prelude::*;
use dioxus_icons::lucide::{Moon, Sun};

use crate::components::button::{Button, ButtonSize, ButtonVariant};

/// Toggles light and dark. Until the first toggle the theme follows the system; the choice is per
/// browser, saved in localStorage under `storage_key`, the `theme_key` of `DocumentHead`.
#[component]
pub fn ThemeSwitch(storage_key: &'static str) -> Element {
    let mut dark = use_signal(|| false);
    use_effect(move || dark.set(current_theme().as_deref() == Some("dark")));

    rsx! {
        Button {
            variant: ButtonVariant::Quiet,
            size: ButtonSize::Icon,
            aria_label: if dark() { "Switch to light theme" } else { "Switch to dark theme" },
            onclick: move |_| {
                let next = if dark() { "light" } else { "dark" };
                save_theme(storage_key, next);
                dark.set(next == "dark");
            },
            if dark() {
                Sun {}
            } else {
                Moon {}
            }
        }
    }
}

/// The `data-theme` of `<html>`.
#[cfg(target_arch = "wasm32")]
fn current_theme() -> Option<String> {
    web_sys::window()?.document()?.document_element()?.get_attribute("data-theme")
}

#[cfg(not(target_arch = "wasm32"))]
fn current_theme() -> Option<String> {
    None
}

/// Saves `theme` under `key` and sets it as the `data-theme` of `<html>`.
#[cfg(target_arch = "wasm32")]
fn save_theme(key: &str, theme: &str) {
    let Some(window) = web_sys::window() else { return };
    if let Ok(Some(storage)) = window.local_storage() {
        _ = storage.set_item(key, theme);
    }
    if let Some(root) = window.document().and_then(|document| document.document_element()) {
        _ = root.set_attribute("data-theme", theme);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_theme(_key: &str, _theme: &str) {}
