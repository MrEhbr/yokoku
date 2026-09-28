use dioxus::prelude::*;
use dioxus_primitives::label::{self, LabelProps};

#[component]
pub fn Label(props: LabelProps) -> Element {
    rsx! {
        label::Label {
            class: "text-caption font-medium",
            html_for: props.html_for,
            attributes: props.attributes,
            {props.children}
        }
    }
}
