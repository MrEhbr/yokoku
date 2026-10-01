use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// A pulsing placeholder for content that is loading. Set its size through classes in
/// `attributes`, matching the expected content's shape.
#[component]
pub fn Skeleton(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let base = attributes!(div { class: "animate-pulse bg-subtle" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { ..merged }
    }
}

/// The content that takes a [`Skeleton`]'s place, faded in when it mounts.
#[component]
pub fn Loaded(children: Element) -> Element {
    rsx! {
        div { class: "animate-fade-in motion-reduce:animate-none", {children} }
    }
}
