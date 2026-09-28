use dioxus::prelude::*;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    progress::{self, ProgressProps},
};

/// A progress bar. `value: None` is indeterminate: use it when the total is unknown, never 0%.
/// Give it an `aria_label`.
#[component]
pub fn Progress(props: ProgressProps) -> Element {
    let base = attributes!(div { class: "group relative h-1.5 w-full overflow-hidden bg-subtle" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        progress::Progress { value: props.value, max: props.max, attributes: merged,
            progress::ProgressIndicator { class: "h-full w-(--progress-value) bg-ink transition-[width] duration-250 ease-interface \
                        motion-reduce:transition-none motion-reduce:animate-none \
                        group-data-[state=indeterminate]:w-1/2 \
                        group-data-[state=indeterminate]:animate-progress-indeterminate" }
        }
    }
}
