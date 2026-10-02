use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use dioxus_primitives::{
    accordion::{self, AccordionContentProps, AccordionItemProps, AccordionProps, AccordionTriggerProps},
    dioxus_attributes::attributes,
    merge_attributes,
};

/// A group of collapsible sections. Items open and close independently unless
/// `allow_multiple_open` is `false`.
#[component]
pub fn Accordion(props: AccordionProps) -> Element {
    let base = attributes!(div { class: "w-full" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        accordion::Accordion {
            id: props.id,
            allow_multiple_open: props.allow_multiple_open,
            disabled: props.disabled,
            collapsible: props.collapsible,
            horizontal: props.horizontal,
            attributes: merged,
            {props.children}
        }
    }
}

/// One collapsible section. Sets `group` so its [`AccordionTrigger`] chevron can read its
/// `data-open` state.
#[component]
pub fn AccordionItem(props: AccordionItemProps) -> Element {
    let base = attributes!(div { class: "group border-b border-line last:border-b-0" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        accordion::AccordionItem {
            disabled: props.disabled,
            default_open: props.default_open,
            on_change: props.on_change,
            on_trigger_click: props.on_trigger_click,
            index: props.index,
            attributes: merged,
            {props.children}
        }
    }
}

/// The heading that opens and closes an [`AccordionItem`].
#[component]
pub fn AccordionTrigger(props: AccordionTriggerProps) -> Element {
    let base = attributes!(button {
        class: "flex w-full cursor-pointer items-center justify-between gap-4 py-4 text-left text-body \
                font-medium transition-colors duration-120 ease-interface hover:text-muted \
                disabled:cursor-not-allowed disabled:opacity-45",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        accordion::AccordionTrigger { id: props.id, attributes: merged,
            {props.children}
            ChevronDown {
                size: "1rem",
                class: "shrink-0 text-muted transition-transform duration-200 ease-interface \
                        group-data-[open=true]:rotate-180",
            }
        }
    }
}

/// The content shown while its [`AccordionItem`] is open. Animates its height with a
/// `grid-template-rows` transition.
#[component]
pub fn AccordionContent(props: AccordionContentProps) -> Element {
    let base = attributes!(div {
        class: "grid overflow-hidden transition-[grid-template-rows] duration-200 ease-interface \
                motion-reduce:transition-none data-[open=true]:grid-rows-[1fr] \
                data-[open=false]:grid-rows-[0fr]",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        accordion::AccordionContent { id: props.id, attributes: merged,
            div { class: "min-h-0 overflow-hidden pb-4 text-body text-muted", {props.children} }
        }
    }
}
