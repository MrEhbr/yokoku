use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    separator::{self, SeparatorProps},
};

/// A thin rule between groups of content. Pass `decorative: true` for a purely visual divider
/// that assistive technology should ignore.
#[component]
pub fn Separator(props: SeparatorProps) -> Element {
    let base = attributes!(div {
        class: "shrink-0 bg-line data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full \
                data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        separator::Separator {
            horizontal: props.horizontal,
            decorative: props.decorative,
            attributes: merged,
            {props.children}
        }
    }
}
