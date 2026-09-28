use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    hover_card::{self, HoverCardContentProps, HoverCardProps, HoverCardTriggerProps},
    merge_attributes,
};

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
                data-[state=open]:animate-popover-in data-[state=closed]:animate-popover-out \
                data-[side=top]:bottom-full data-[side=top]:mb-2 \
                data-[side=bottom]:top-full data-[side=bottom]:mt-2 \
                data-[side=left]:right-full data-[side=left]:mr-2 \
                data-[side=right]:left-full data-[side=right]:ml-2 \
                data-[side=top]:data-[align=center]:left-1/2 data-[side=top]:data-[align=center]:-translate-x-1/2 \
                data-[side=bottom]:data-[align=center]:left-1/2 data-[side=bottom]:data-[align=center]:-translate-x-1/2 \
                data-[side=top]:data-[align=start]:left-0 data-[side=bottom]:data-[align=start]:left-0 \
                data-[side=top]:data-[align=end]:right-0 data-[side=bottom]:data-[align=end]:right-0 \
                data-[side=left]:data-[align=center]:top-1/2 data-[side=left]:data-[align=center]:-translate-y-1/2 \
                data-[side=right]:data-[align=center]:top-1/2 data-[side=right]:data-[align=center]:-translate-y-1/2 \
                data-[side=left]:data-[align=start]:top-0 data-[side=right]:data-[align=start]:top-0 \
                data-[side=left]:data-[align=end]:bottom-0 data-[side=right]:data-[align=end]:bottom-0",
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
