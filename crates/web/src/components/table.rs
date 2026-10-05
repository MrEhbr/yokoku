//! Dense data rows with quiet rules. Select rows with checkboxes, never by clicking the row.

use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// Scrolls sideways inside its own box when the columns don't fit.
#[component]
pub fn Table(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(table { class: "w-full caption-bottom border-collapse text-left text-body" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        div { class: "w-full overflow-x-auto",
            table { ..merged,{children} }
        }
    }
}

#[component]
pub fn TableHeader(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    rsx! {
        thead { ..attributes,{children} }
    }
}

#[component]
pub fn TableBody(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    rsx! {
        tbody { ..attributes,{children} }
    }
}

/// Set `"data-selected": true` on a checked row.
#[component]
pub fn TableRow(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(tr { class: "border-b border-line data-[selected=true]:bg-subtle" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        tr { ..merged,{children} }
    }
}

#[component]
pub fn TableHead(
    #[props(extends = GlobalAttributes)]
    #[props(extends = th)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(th {
        class: "px-3 py-2 text-left align-middle text-caption font-medium whitespace-nowrap text-muted",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        th { ..merged,{children} }
    }
}

#[component]
pub fn TableCell(
    #[props(extends = GlobalAttributes)]
    #[props(extends = td)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(td { class: "px-3 py-3 align-middle" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        td { ..merged,{children} }
    }
}

#[component]
pub fn TableCaption(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>, children: Element) -> Element {
    let base = attributes!(caption { class: "mt-4 text-body text-muted" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        caption { ..merged,{children} }
    }
}

/// Where a sortable column stands; `None` while another column sorts the table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// A column header whose button sorts the table. It announces the direction with `aria-sort`.
#[component]
pub fn TableSortHead(
    label: String,
    direction: Option<SortDirection>,
    onclick: EventHandler<MouseEvent>,
    #[props(default)] class: String,
) -> Element {
    let (aria_sort, arrow) = match direction {
        Some(SortDirection::Ascending) => (Some("ascending"), "↑"),
        Some(SortDirection::Descending) => (Some("descending"), "↓"),
        None => (None, ""),
    };
    rsx! {
        TableHead { class, aria_sort,
            button {
                r#type: "button",
                class: "inline-flex cursor-pointer items-center gap-1 hover:text-ink",
                onclick: move |event| onclick.call(event),
                "{label}"
                span { aria_hidden: "true", "{arrow}" }
            }
        }
    }
}
