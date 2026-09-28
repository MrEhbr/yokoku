use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// A keyboard key label rendered as `<kbd>`. Pass the key name as children and use [`KbdGroup`]
/// for a shortcut with several keys.
#[component]
pub fn Kbd(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(kbd {
        class: "inline-flex h-5 w-fit min-w-5 shrink-0 items-center justify-center gap-1 border \
                border-line bg-subtle px-1.5 font-mono text-caption font-medium text-muted",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        kbd { ..merged,{children} }
    }
}

/// A row of key labels for one keyboard shortcut. The keys stay on one line.
#[component]
pub fn KbdGroup(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(span { class: "inline-flex items-center gap-1 whitespace-nowrap" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        span { ..merged,{children} }
    }
}
