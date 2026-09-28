use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, Minus};
pub use dioxus_primitives::checkbox::CheckboxState;
use dioxus_primitives::{
    checkbox::{self, CheckboxProps},
    dioxus_attributes::attributes,
    merge_attributes,
};

/// Give it an `aria_label`, or wrap it in a `Label`.
#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex size-4 shrink-0 cursor-pointer items-center justify-center border border-control \
                bg-surface p-0 text-canvas transition-colors \
                data-[state=checked]:border-ink data-[state=checked]:bg-ink \
                data-[state=indeterminate]:border-ink data-[state=indeterminate]:bg-ink \
                data-[disabled=true]:cursor-not-allowed data-[disabled=true]:opacity-45",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        checkbox::Checkbox {
            checked: props.checked,
            default_checked: props.default_checked,
            required: props.required,
            disabled: props.disabled,
            name: props.name,
            value: props.value,
            on_checked_change: props.on_checked_change,
            attributes: merged,
            checkbox::CheckboxIndicator { class: "group flex items-center justify-center",
                Check {
                    size: "0.875rem",
                    class: "group-data-[state=indeterminate]:hidden",
                }
                Minus {
                    size: "0.875rem",
                    class: "hidden group-data-[state=indeterminate]:block",
                }
            }
        }
    }
}
