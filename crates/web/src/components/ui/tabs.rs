use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    tabs::{self, TabContentProps, TabListProps, TabTriggerProps, TabsProps},
};

/// A group of panels with buttons that switch the visible panel.
///
/// Pass `horizontal: true` for a left-to-right row of tabs; it also picks the arrow-key
/// direction for keyboard navigation.
#[component]
pub fn Tabs(props: TabsProps) -> Element {
    let base = attributes!(div { class: "flex flex-col gap-4" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        tabs::Tabs {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TabList(props: TabListProps) -> Element {
    let base = attributes!(div { class: "flex flex-wrap border-b border-line" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        tabs::TabList { attributes: merged, {props.children} }
    }
}

/// A tab button. Selected styling follows the kit's `.yk-tab` recipe: an ink underline.
#[component]
pub fn TabTrigger(props: TabTriggerProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex min-h-9 shrink-0 cursor-pointer items-center justify-center gap-2 \
                border-b-2 border-transparent px-3 py-2 text-body whitespace-nowrap text-muted \
                transition-colors hover:bg-subtle aria-selected:border-ink aria-selected:text-ink \
                disabled:cursor-not-allowed disabled:opacity-45",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        tabs::TabTrigger {
            class: None,
            id: props.id,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

/// The panel shown for the selected [`TabTrigger`].
#[component]
pub fn TabContent(props: TabContentProps) -> Element {
    let base = attributes!(div { class: "pt-4" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        tabs::TabContent {
            class: None,
            value: props.value,
            id: props.id,
            index: props.index,
            attributes: merged,
            {props.children}
        }
    }
}
