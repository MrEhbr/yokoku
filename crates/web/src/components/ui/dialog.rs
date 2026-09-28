use dioxus::prelude::*;
use dioxus_primitives::{
    dialog::{self, DialogDescriptionProps, DialogRootProps, DialogTitleProps},
    dioxus_attributes::attributes,
    merge_attributes,
};

/// A modal panel over an overlay. Escape and a click outside close it through `on_open_change`;
/// focus stays inside while open.
///
/// Label it with a [`DialogTitle`]. Closing discards what the dialog's children hold, so keep
/// uncommitted state that must survive outside.
#[component]
pub fn Dialog(props: DialogRootProps) -> Element {
    let base = attributes!(div {
        class: "relative my-auto flex max-h-[calc(100dvh-2rem)] w-full max-w-dialog flex-col gap-4 overflow-y-auto \
                border border-ink bg-surface p-5 text-ink shadow-dialog",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogRoot {
            class: "fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-overlay p-4 \
                    data-[state=open]:animate-fade-in data-[state=closed]:animate-fade-out",
            id: props.id,
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            dialog::DialogContent { class: None, attributes: merged, {props.children} }
        }
    }
}

#[component]
pub fn DialogTitle(props: DialogTitleProps) -> Element {
    let base = attributes!(h2 { class: "text-section font-medium" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogTitle { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DialogDescription(props: DialogDescriptionProps) -> Element {
    let base = attributes!(p { class: "text-body text-muted" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogDescription { id: props.id, attributes: merged, {props.children} }
    }
}

/// The dialog's actions, aligned right; they wrap when they don't fit.
#[component]
pub fn DialogFooter(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "flex flex-wrap items-center justify-end gap-2 border-t border-line pt-4" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { ..merged,{children} }
    }
}
