use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    dropdown_menu::{
        self, DropdownMenuContentProps, DropdownMenuItemProps, DropdownMenuProps, DropdownMenuTriggerProps,
    },
    merge_attributes,
};

#[component]
pub fn DropdownMenu(props: DropdownMenuProps) -> Element {
    let base = attributes!(div { class: "relative inline-block" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dropdown_menu::DropdownMenu {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            disabled: props.disabled,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

/// Give it Paper button classes through `attrs` to look like a button; a `group-data-[state=open]:`
/// variant on a child icon can react to the open state.
#[component]
pub fn DropdownMenuTrigger(props: DropdownMenuTriggerProps) -> Element {
    let base = attributes!(button {
        class: "group inline-flex cursor-pointer items-center gap-2 disabled:cursor-not-allowed disabled:opacity-45",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dropdown_menu::DropdownMenuTrigger { r#as: props.r#as, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DropdownMenuContent(props: DropdownMenuContentProps) -> Element {
    let base = attributes!(div {
        class: "absolute top-full left-0 z-50 mt-1 min-w-40 border border-control bg-surface p-1 text-ink \
                shadow-popover data-[state=open]:animate-popover-in data-[state=closed]:animate-popover-out",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dropdown_menu::DropdownMenuContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DropdownMenuItem<T: Clone + PartialEq + 'static>(props: DropdownMenuItemProps<T>) -> Element {
    let base = attributes!(div {
        class: "flex w-full cursor-pointer items-center gap-2 px-2 py-1.5 text-left text-body whitespace-nowrap \
                select-none outline-none hover:bg-subtle focus:bg-subtle \
                data-[disabled=true]:pointer-events-none data-[disabled=true]:text-muted",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dropdown_menu::DropdownMenuItem {
            disabled: props.disabled,
            value: props.value,
            index: props.index,
            on_select: props.on_select,
            attributes: merged,
            {props.children}
        }
    }
}
