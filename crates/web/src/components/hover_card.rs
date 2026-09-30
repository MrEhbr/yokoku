use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    hover_card::{self, HoverCardContentProps, HoverCardProps, HoverCardTriggerProps},
    merge_attributes,
};

use super::tooltip::POPOVER_PLACEMENT;

#[component]
pub fn HoverCard(props: HoverCardProps) -> Element {
    let base = attributes!(div { class: "group relative inline-block" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        hover_card::HoverCard {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn HoverCardTrigger(props: HoverCardTriggerProps) -> Element {
    let base = attributes!(div {
        class: "inline-block group-data-[disabled=true]:cursor-default group-data-[disabled=true]:text-muted",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        hover_card::HoverCardTrigger { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn HoverCardContent(props: HoverCardContentProps) -> Element {
    let base = attributes!(div {
        class: "absolute z-50 w-64 border border-control bg-surface p-4 text-ink shadow-popover \
                {POPOVER_PLACEMENT}",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        hover_card::HoverCardContent {
            id: props.id,
            side: props.side,
            align: props.align,
            force_mount: props.force_mount,
            attributes: merged,
            {props.children}
        }
    }
}
