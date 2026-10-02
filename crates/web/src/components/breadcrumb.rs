//! Navigation links showing the path to the current page. No upstream primitive.

use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronRight, Ellipsis};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// Place links in a [`BreadcrumbList`] and give the current page a [`BreadcrumbPage`]
/// instead of a link.
#[component]
pub fn Breadcrumb(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    rsx! {
        nav { aria_label: "breadcrumb", ..attributes, {children} }
    }
}

/// The ordered list of steps in a [`Breadcrumb`]. Wraps onto another line rather than
/// overflowing when the trail outgrows its container.
#[component]
pub fn BreadcrumbList(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(ol { class: "flex flex-wrap items-center gap-2 text-body text-muted" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        ol { ..merged,{children} }
    }
}

/// One step of a [`BreadcrumbList`], holding a [`BreadcrumbLink`] or a [`BreadcrumbPage`].
#[component]
pub fn BreadcrumbItem(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(li { class: "inline-flex items-center gap-2" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        li { ..merged,{children} }
    }
}

/// A link to an ancestor page. Pass its destination as `href` in `attributes`.
#[component]
pub fn BreadcrumbLink(
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(a { class: "transition-colors hover:text-ink" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        a { ..merged,{children} }
    }
}

/// The current page's label. Renders with `aria-current="page"` and does not navigate.
#[component]
pub fn BreadcrumbPage(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(span { class: "font-medium text-ink" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        span { aria_current: "page", ..merged, {children} }
    }
}

/// A decorative chevron between breadcrumb items, hidden from assistive technology.
#[component]
pub fn BreadcrumbSeparator(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    rsx! {
        li { aria_hidden: "true", ..attributes,
            ChevronRight { size: "0.875rem" }
        }
    }
}

/// An ellipsis representing omitted breadcrumb items, with an accessible text label.
#[component]
pub fn BreadcrumbEllipsis(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let base = attributes!(span { class: "flex items-center" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        span {..merged,
            Ellipsis { size: "1rem" }
            span { class: "sr-only", "More" }
        }
    }
}
