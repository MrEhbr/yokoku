use dioxus::prelude::*;
use dioxus_primitives::toggle::{self, ToggleProps};

/// A control that stays pressed when selected. Give it an `aria_label` if its children are icon-only.
#[component]
pub fn Toggle(props: ToggleProps) -> Element {
    rsx! {
        toggle::Toggle {
            class: "inline-flex min-h-9 shrink-0 cursor-pointer items-center justify-center gap-2 border \
                    border-transparent px-3 text-body font-medium whitespace-nowrap transition-colors \
                    duration-120 ease-interface motion-reduce:transition-none select-none text-muted \
                    hover:not-disabled:bg-subtle hover:not-disabled:text-ink \
                    data-[state=on]:bg-subtle data-[state=on]:text-ink \
                    disabled:pointer-events-none disabled:opacity-45",
            pressed: props.pressed,
            default_pressed: props.default_pressed,
            disabled: props.disabled,
            on_pressed_change: props.on_pressed_change,
            onmounted: props.onmounted,
            onfocus: props.onfocus,
            onkeydown: props.onkeydown,
            attributes: props.attributes,
            {props.children}
        }
    }
}
