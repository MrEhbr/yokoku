use dioxus::prelude::*;
use dioxus_icons::lucide::LoaderCircle;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// An animated icon for work in progress. The default size matches the surrounding text.
///
/// ```rust,ignore
/// rsx! {
///     Button { disabled: true, aria_busy: "true",
///         Spinner {}
///         "Importing"
///     }
/// }
/// ```
#[component]
pub fn Spinner(
    #[props(into, default = "1em".to_owned())] size: String,
    #[props(into, default = "Loading".to_owned())] label: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(svg { role: "img", "aria-label": label, class: "animate-spin motion-reduce:animate-none" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        LoaderCircle { size, attributes: merged }
    }
}
