use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight, Ellipsis};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// Classes shown only once the pagination nav is wide enough for the label text.
const LABEL: &str = "sr-only @xs:not-sr-only";

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum PaginationLinkSize {
    #[default]
    Icon,
    Default,
}

#[derive(Copy, Clone, PartialEq)]
#[non_exhaustive]
pub enum PaginationLinkKind {
    Previous,
    Next,
}

impl PaginationLinkKind {
    fn attr(self) -> &'static str {
        match self {
            Self::Previous => "previous",
            Self::Next => "next",
        }
    }
}

/// Classes for a page link's active state and size.
fn link_class(is_active: bool, size: PaginationLinkSize) -> &'static str {
    match (is_active, size) {
        (true, PaginationLinkSize::Icon) => "size-9 border-control text-ink hover:bg-subtle [&>svg]:size-4",
        (false, PaginationLinkSize::Icon) => {
            "size-9 border-transparent text-muted hover:bg-subtle hover:text-ink [&>svg]:size-4"
        },
        (true, PaginationLinkSize::Default) => "gap-1.5 border-control px-3 py-1.5 text-ink hover:bg-subtle",
        (false, PaginationLinkSize::Default) => {
            "gap-1.5 border-transparent px-3 py-1.5 text-muted hover:bg-subtle hover:text-ink"
        },
    }
}

/// Links for a list split across pages.
#[component]
pub fn Pagination(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(nav {
        "data-slot": "pagination",
        class: "@container mx-auto flex w-full justify-center",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        nav { role: "navigation", aria_label: "pagination", ..merged, {children} }
    }
}

#[component]
pub fn PaginationContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(ul {
        "data-slot": "pagination-content",
        class: "flex flex-row flex-wrap items-center justify-center gap-1",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        ul { ..merged,{children} }
    }
}

#[component]
pub fn PaginationItem(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    rsx! {
        li { "data-slot": "pagination-item", ..attributes, {children} }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct PaginationLinkProps {
    #[props(default)]
    pub is_active: bool,
    #[props(default)]
    pub size: PaginationLinkSize,
    #[props(default)]
    pub data_kind: Option<PaginationLinkKind>,
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

/// A link to one page. `is_active` marks the page being read.
#[component]
pub fn PaginationLink(props: PaginationLinkProps) -> Element {
    let aria_current = if props.is_active { Some("page") } else { None };
    let data_kind = props.data_kind.map(PaginationLinkKind::attr);
    let variant = link_class(props.is_active, props.size);
    let base = attributes!(a {
        "data-slot": "pagination-link",
        class: "inline-flex min-h-9 shrink-0 cursor-pointer items-center justify-center border text-body \
                font-medium whitespace-nowrap transition-colors {variant}",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        a {
            "data-active": props.is_active,
            "data-size": match props.size {
                PaginationLinkSize::Icon => "icon",
                PaginationLinkSize::Default => "default",
            },
            "data-kind": data_kind,
            aria_current,
            onclick: move |event| {
                if let Some(f) = &props.onclick {
                    f.call(event);
                }
            },
            onmousedown: move |event| {
                if let Some(f) = &props.onmousedown {
                    f.call(event);
                }
            },
            onmouseup: move |event| {
                if let Some(f) = &props.onmouseup {
                    f.call(event);
                }
            },
            ..merged,
            {props.children}
        }
    }
}

#[component]
pub fn PaginationPrevious(
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        PaginationLink {
            size: PaginationLinkSize::Default,
            aria_label: "Go to previous page",
            data_kind: Some(PaginationLinkKind::Previous),
            onclick,
            onmousedown,
            onmouseup,
            attributes,
            ChevronLeft { size: "1rem" }
            span { class: LABEL, "Previous" }
        }
    }
}

#[component]
pub fn PaginationNext(
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        PaginationLink {
            size: PaginationLinkSize::Default,
            aria_label: "Go to next page",
            data_kind: Some(PaginationLinkKind::Next),
            onclick,
            onmousedown,
            onmouseup,
            attributes,
            span { class: LABEL, "Next" }
            ChevronRight { size: "1rem" }
        }
    }
}

#[component]
pub fn PaginationEllipsis(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let base = attributes!(span {
        "data-slot": "pagination-ellipsis",
        class: "flex size-9 items-center justify-center text-muted [&>svg]:size-4",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        span { aria_hidden: "true", ..merged,
            Ellipsis { size: "1rem" }
            span { class: "sr-only", "More pages" }
        }
    }
}
