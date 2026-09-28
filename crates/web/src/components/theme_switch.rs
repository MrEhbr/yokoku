use dioxus::prelude::*;
use dioxus_icons::lucide::{Moon, Sun};

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};

/// Toggles light and dark. Until the first toggle the theme follows the system; the choice is per browser.
#[component]
pub fn ThemeSwitch() -> Element {
    let mut dark = use_signal(|| false);
    use_effect(move || {
        spawn(async move {
            let theme = document::eval("return document.documentElement.dataset.theme");
            if let Ok(theme) = theme.join::<String>().await {
                dark.set(theme == "dark");
            }
        });
    });

    rsx! {
        Button {
            variant: ButtonVariant::Quiet,
            size: ButtonSize::Icon,
            aria_label: if dark() { "Switch to light theme" } else { "Switch to dark theme" },
            onclick: move |_| async move {
                if let Ok(theme) = document::eval("return ykToggleTheme()")
                    .join::<String>()
                    .await
                {
                    dark.set(theme == "dark");
                }
            },
            if dark() {
                Sun {}
            } else {
                Moon {}
            }
        }
    }
}
