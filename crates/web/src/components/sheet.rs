use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use dioxus_primitives::{
    dialog::{self, DialogCtx, DialogDescriptionProps, DialogRootProps, DialogTitleProps},
    dioxus_attributes::attributes,
    merge_attributes,
};

/// The edge a sheet's content slides in from. Set it through the `data-side`
/// attribute on [`Sheet`]; the CSS reads that attribute, not a prop.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SheetSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

impl SheetSide {
    pub fn as_str(self) -> &'static str {
        match self {
            SheetSide::Top => "top",
            SheetSide::Right => "right",
            SheetSide::Bottom => "bottom",
            SheetSide::Left => "left",
        }
    }
}

/// Position, border and motion for every `data-side`. `in-[[data-state=…]]` reads the
/// state the [`Sheet`] root sets.
const CONTENT: &str = "fixed z-50 flex flex-col gap-4 overflow-y-auto border-ink bg-surface text-ink shadow-dialog \
    data-[side=right]:inset-y-0 data-[side=right]:right-0 data-[side=right]:w-3/4 data-[side=right]:max-w-sm data-[side=right]:border-l \
    data-[side=left]:inset-y-0 data-[side=left]:left-0 data-[side=left]:w-3/4 data-[side=left]:max-w-sm data-[side=left]:border-r \
    data-[side=top]:inset-x-0 data-[side=top]:top-0 data-[side=top]:max-h-full data-[side=top]:border-b \
    data-[side=bottom]:inset-x-0 data-[side=bottom]:bottom-0 data-[side=bottom]:max-h-full data-[side=bottom]:border-t \
    data-[side=right]:in-[[data-state=open]]:animate-slide-in-right data-[side=right]:in-[[data-state=closed]]:animate-slide-out-right \
    data-[side=left]:in-[[data-state=open]]:animate-slide-in-left data-[side=left]:in-[[data-state=closed]]:animate-slide-out-left \
    data-[side=top]:in-[[data-state=open]]:animate-slide-in-top data-[side=top]:in-[[data-state=closed]]:animate-slide-out-top \
    data-[side=bottom]:in-[[data-state=open]]:animate-slide-in-bottom data-[side=bottom]:in-[[data-state=closed]]:animate-slide-out-bottom";

#[component]
pub fn Sheet(props: DialogRootProps) -> Element {
    let content_base = attributes!(div {
        class: CONTENT,
        "data-slot": "sheet-content",
        "data-side": SheetSide::Right.as_str(),
    });
    let content_attributes = merge_attributes(vec![content_base, props.attributes]);

    rsx! {
        dialog::DialogRoot {
            class: "fixed inset-0 z-50 bg-overlay data-[state=open]:animate-fade-in data-[state=closed]:animate-fade-out",
            "data-slot": "sheet-root",
            id: props.id,
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            dialog::DialogContent { class: None, attributes: content_attributes, {props.children} }
        }
    }
}

#[component]
pub fn SheetContentClose(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let base = attributes!(button {
        class: "absolute right-4 top-4 flex size-6 cursor-pointer items-center justify-center text-muted \
                transition-colors hover:bg-subtle hover:text-ink",
        aria_label: "Close",
    });
    let attributes = merge_attributes(vec![base, attributes]);

    rsx! {
        SheetClose { attributes,
            X { size: "1rem" }
        }
    }
}

#[component]
pub fn SheetHeader(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "flex flex-col gap-1.5 p-4", "data-slot": "sheet-header" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { ..merged,{children} }
    }
}

#[component]
pub fn SheetFooter(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "mt-auto flex flex-col gap-2 p-4", "data-slot": "sheet-footer" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { ..merged,{children} }
    }
}

#[component]
pub fn SheetTitle(props: DialogTitleProps) -> Element {
    let base = attributes!(h2 { class: "text-section font-medium", "data-slot": "sheet-title" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogTitle { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn SheetDescription(props: DialogDescriptionProps) -> Element {
    let base = attributes!(p { class: "text-body text-muted", "data-slot": "sheet-description" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        dialog::DialogDescription { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn SheetClose(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let ctx: DialogCtx = use_context();

    let base = attributes! {
        button {
            onclick: move |_| {
                ctx.set_open(false);
            }
        }
    };
    let merged = merge_attributes(vec![base, attributes]);

    if let Some(dynamic) = r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            button { ..merged,{children} }
        }
    }
}
