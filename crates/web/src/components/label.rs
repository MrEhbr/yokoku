use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    label::{self, LabelProps},
    merge_attributes,
};

#[component]
pub fn Label(props: LabelProps) -> Element {
    let base = attributes!(label { class: "text-caption font-medium" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        label::Label { html_for: props.html_for, attributes: merged, {props.children} }
    }
}
