use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    radio_group::{self, RadioGroupProps, RadioItemProps},
};

/// A container for radio items that allow selecting one option from a list.
///
/// Name the group with `aria_label` or `aria_labelledby`.
#[component]
pub fn RadioGroup(props: RadioGroupProps) -> Element {
    let base = attributes!(div {
        class: "flex flex-col gap-3 data-[orientation=horizontal]:flex-row \
                data-[orientation=horizontal]:flex-wrap data-[orientation=horizontal]:gap-4",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        radio_group::RadioGroup {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            required: props.required,
            name: props.name,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

/// One option in a [`RadioGroup`]. Its children are the option's own label text.
#[component]
pub fn RadioItem(props: RadioItemProps) -> Element {
    let base = attributes!(button {
        class: "group inline-flex w-fit cursor-pointer items-center gap-3 border-none bg-transparent p-0 \
                text-left text-body text-ink disabled:cursor-not-allowed disabled:opacity-45",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        radio_group::RadioItem {
            class: None,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            id: props.id,
            attributes: merged,
            span {
                aria_hidden: "true",
                class: "relative inline-flex size-4 shrink-0 items-center justify-center rounded-full border \
                        border-control bg-surface transition-colors group-data-[state=checked]:border-ink",
                span { class: "size-2 rounded-full bg-ink opacity-0 transition-opacity group-data-[state=checked]:opacity-100" }
            }
            {props.children}
        }
    }
}
