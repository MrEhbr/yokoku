use dioxus::prelude::*;
use dioxus_icons::lucide::ArrowLeft;

use crate::{
    components::button::{Button, ButtonSize, ButtonVariant},
    route::Route,
};

/// Whether the route changed since the app loaded: then the browser's previous page is one of ours.
#[derive(Clone, Copy)]
struct InAppHistory(Signal<bool>);

/// Records in-app navigation for [`BackButton`]; the shell calls it once.
pub(super) fn use_in_app_history() {
    let route = use_route::<Route>();
    let start = use_hook(|| route.clone());
    let InAppHistory(mut moved) = use_context_provider(|| InAppHistory(Signal::new(false)));
    use_effect(use_reactive!(|route| {
        if route != start && !*moved.peek() {
            moved.set(true);
        }
    }));
}

/// Returns to the previous page, or opens `fallback` on a page opened directly.
#[component]
pub fn BackButton(fallback: Route) -> Element {
    let InAppHistory(moved) = use_context();
    let navigator = navigator();
    rsx! {
        Button {
            variant: ButtonVariant::Quiet,
            size: ButtonSize::Sm,
            class: "-ml-2.5",
            onclick: move |_| {
                if moved() {
                    navigator.go_back();
                } else {
                    navigator.push(fallback.clone());
                }
            },
            ArrowLeft { size: "1rem" }
            "Back"
        }
    }
}
