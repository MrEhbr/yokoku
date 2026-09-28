use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// The visual style of an [`Alert`].
#[derive(Copy, Clone, PartialEq, Default)]
pub enum AlertVariant {
    /// A plain notice.
    #[default]
    Neutral,
    /// An informational or upcoming state.
    Info,
    /// Something that needs review.
    Warning,
    /// A failure, with the reason and a way to retry.
    Danger,
}

impl AlertVariant {
    /// Classes for the variant's rule, fill, and text colors.
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => "border-control bg-subtle text-ink",
            Self::Info => "border-info bg-info-soft text-info",
            Self::Warning => "border-warning bg-warning-soft text-warning",
            Self::Danger => "border-danger bg-danger-soft text-danger",
        }
    }
}

/// A notice displayed within the page. Pass an optional icon, an [`AlertTitle`], and an
/// [`AlertDescription`] as children. No `role="alert"`; add it in `attributes` when the message
/// must interrupt a screen reader immediately.
#[component]
pub fn Alert(
    #[props(default)] variant: AlertVariant,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let variant = variant.class();
    let base = attributes!(div {
        class: "grid w-full grid-cols-[0_1fr] items-start gap-y-0.5 border-l-2 px-3 py-2 text-body \
                has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-2.5 [&>svg]:size-4 [&>svg]:translate-y-0.5 {variant}",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { ..merged,{children} }
    }
}

/// The heading of an [`Alert`].
#[component]
pub fn AlertTitle(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(p { class: "col-start-2 font-medium tracking-tight" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        p { ..merged,{children} }
    }
}

/// Text that explains the alert and any action the reader should take.
#[component]
pub fn AlertDescription(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(div { class: "col-start-2 text-body" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        div { ..merged,{children} }
    }
}
