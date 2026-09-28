use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// A bordered panel that groups related content. Compose a [`CardHeader`], [`CardContent`], and
/// [`CardFooter`] as needed.
#[component]
pub fn Card(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "flex flex-col gap-4 border border-line bg-surface py-5 text-ink" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { "data-slot": "card", ..merged, {children} }
    }
}

/// The opening section of a [`Card`], stacking a [`CardTitle`] and an optional
/// [`CardDescription`]. Widens to a two-column grid when it holds a [`CardAction`].
#[component]
pub fn CardHeader(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div {
        class: "grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-5 \
                has-data-[slot=card-action]:grid-cols-[1fr_auto]",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { "data-slot": "card-header", ..merged, {children} }
    }
}

/// The heading of a [`Card`], rendered as an `<h3>`.
#[component]
pub fn CardTitle(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(h3 { class: "text-section font-medium" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        h3 { "data-slot": "card-title", ..merged, {children} }
    }
}

/// The supporting text under a [`CardTitle`].
#[component]
pub fn CardDescription(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(p { class: "text-body text-muted" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        p { "data-slot": "card-description", ..merged, {children} }
    }
}

/// An action anchored to the top right of a [`CardHeader`], beside its title and description.
#[component]
pub fn CardAction(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "col-start-2 row-span-2 row-start-1 self-start justify-self-end" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { "data-slot": "card-action", ..merged, {children} }
    }
}

/// The main body of a [`Card`].
#[component]
pub fn CardContent(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "px-5" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { "data-slot": "card-content", ..merged, {children} }
    }
}

/// The closing section of a [`Card`], a horizontal row for actions.
#[component]
pub fn CardFooter(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "flex flex-wrap items-center gap-2 px-5" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { "data-slot": "card-footer", ..merged, {children} }
    }
}
