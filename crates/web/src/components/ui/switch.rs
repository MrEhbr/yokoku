use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    switch::{self, SwitchProps},
};

/// An on/off control for a setting. Give it an `aria_label`, or wrap it in a `Label`.
#[component]
pub fn Switch(props: SwitchProps) -> Element {
    let base = attributes!(button {
        class: "group relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center border \
                border-control bg-subtle transition-colors data-[state=checked]:border-ink \
                data-[state=checked]:bg-ink disabled:cursor-not-allowed disabled:opacity-45",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        switch::Switch {
            checked: props.checked,
            default_checked: props.default_checked,
            disabled: props.disabled,
            required: props.required,
            name: props.name,
            value: props.value,
            on_checked_change: props.on_checked_change,
            attributes: merged,
            switch::SwitchThumb { class: "pointer-events-none absolute top-1/2 left-[3px] size-3.5 -translate-y-1/2 bg-control \
                        transition-transform group-data-[state=checked]:translate-x-4 \
                        group-data-[state=checked]:bg-canvas motion-reduce:transition-none" }
        }
    }
}
