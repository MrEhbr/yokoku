//! A form field: a `Label`, one control, then an optional hint or error.
//!
//! An invalid control sets `aria_invalid: "true"` and `aria_describedby` to its `FieldError`'s id.
//! The error says how to fix the value, not only that it is wrong.

use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Field(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "grid min-w-0 gap-1.5" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        div { ..merged,{children} }
    }
}

#[component]
pub fn FieldHint(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(p { class: "text-caption text-muted" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        p { ..merged,{children} }
    }
}

#[component]
pub fn FieldError(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(p { class: "text-caption text-danger" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        p { ..merged,{children} }
    }
}
