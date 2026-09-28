use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    tooltip::{self, TooltipContentProps, TooltipProps, TooltipTriggerProps},
};

#[component]
pub fn Tooltip(props: TooltipProps) -> Element {
    let base = attributes!(div { class: "group relative inline-block" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        tooltip::Tooltip {
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TooltipTrigger(props: TooltipTriggerProps) -> Element {
    let base = attributes!(div { class: "inline-block group-data-[disabled=true]:cursor-default" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        tooltip::TooltipTrigger { id: props.id, r#as: props.r#as, attributes: merged, {props.children} }
    }
}

#[component]
pub fn TooltipContent(props: TooltipContentProps) -> Element {
    let base = attributes!(div {
        class: "pointer-events-none absolute z-50 max-w-60 bg-ink px-2.5 py-1 text-caption font-medium text-canvas \
                whitespace-nowrap \
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
        tooltip::TooltipContent {
            id: props.id,
            side: props.side,
            align: props.align,
            attributes: merged,
            {props.children}
        }
    }
}
