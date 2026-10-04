use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;

/// A section that opens and closes under its `summary`, as the accordion looks. A native
/// `details`, so it renders on the server and toggles before the page hydrates. Its children
/// render from the first time it opens: on the server only when `open`. `lead`, such as a
/// control, sits before the summary without opening it. Below `sm` the children also span the
/// space under `lead`, which must be 2.25rem wide.
#[component]
pub fn Disclosure(
    #[props(default)] open: bool,
    #[props(default)] lead: Option<Element>,
    summary: Element,
    children: Element,
) -> Element {
    let mut opened = use_signal(|| open);
    let content = if lead.is_some() { "pb-4 max-sm:-ml-11" } else { "pb-4" };
    rsx! {
        div { class: "flex items-start gap-2 border-b border-line",
            if let Some(lead) = lead {
                div { class: "shrink-0 pt-2", {lead} }
            }
            details { class: "group min-w-0 flex-1", open,
                summary {
                    class: "flex cursor-pointer list-none items-center justify-between gap-4 py-4 hover:text-muted [&::-webkit-details-marker]:hidden",
                    onclick: move |_| opened.set(true),
                    {summary}
                    ChevronDown {
                        size: "1rem",
                        class: "shrink-0 text-muted transition-transform duration-200 ease-interface group-open:rotate-180 motion-reduce:transition-none",
                    }
                }
                if opened() {
                    div { class: content, {children} }
                }
            }
        }
    }
}
