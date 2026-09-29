use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;

/// A section that opens and closes under its `summary`, as the accordion looks. A native
/// `details`, so it renders on the server and works before the page hydrates. `lead`, such as a
/// control, sits before the summary without opening it.
#[component]
pub fn Disclosure(
    #[props(default)] open: bool,
    #[props(default)] lead: Option<Element>,
    summary: Element,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex items-start gap-2 border-b border-line",
            if let Some(lead) = lead {
                div { class: "shrink-0 pt-2", {lead} }
            }
            details { class: "group min-w-0 flex-1", open,
                summary { class: "flex cursor-pointer list-none items-center justify-between gap-4 py-4 hover:text-muted [&::-webkit-details-marker]:hidden",
                    {summary}
                    ChevronDown {
                        size: "1rem",
                        class: "shrink-0 text-muted transition-transform duration-200 ease-interface group-open:rotate-180 motion-reduce:transition-none",
                    }
                }
                div { class: "pb-4", {children} }
            }
        }
    }
}
