use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, ChevronDown};
use dioxus_primitives::{
    combobox::{self, ComboboxEmptyProps, ComboboxOptionProps},
    dioxus_attributes::attributes,
    merge_attributes,
};

use super::select::CONTAIN_ESCAPE;

/// A single-choice picker over typed values whose options filter as the user types.
///
/// Closed, the input shows the chosen option's text; a click empties it to search. `id` and
/// `aria_describedby` go to the input, so a `Label` with `html_for` names it. Put a
/// [`ComboboxEmpty`] among the options for a query that matches none.
#[component]
pub fn Combobox<T: Clone + PartialEq + 'static>(
    #[props(default)] value: Option<ReadSignal<Option<T>>>,
    #[props(default)] on_value_change: Callback<Option<T>>,
    #[props(default)] disabled: ReadSignal<bool>,
    #[props(into, default = "Search…".to_owned())] placeholder: String,
    #[props(default)] id: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "group relative block w-full" });
    let merged = merge_attributes(vec![base, attributes]);
    use_effect(|| _ = document::eval(CONTAIN_ESCAPE));
    let mut open = use_signal(|| false);
    let mut query = use_signal(String::new);
    let open_state = use_memo(move || Some(open()));
    let query_state = use_memo(move || Some(query()));

    rsx! {
        combobox::Combobox {
            value,
            on_value_change,
            disabled,
            open: open_state,
            on_open_change: move |next| open.set(next),
            query: query_state,
            on_query_change: move |next| query.set(next),
            attributes: merged,
            div {
                class: "relative",
                // Empties the input before the browser places the caret in the chosen text.
                onpointerdown: move |_| {
                    if !open() && !disabled() {
                        query.set(String::new());
                        open.set(true);
                    }
                },
                combobox::ComboboxInput {
                    class: "min-h-9 w-full min-w-0 border border-control bg-surface py-1.5 pr-8 pl-2 text-body text-ink \
                            placeholder:text-muted disabled:cursor-not-allowed disabled:bg-subtle disabled:text-muted",
                    id,
                    placeholder,
                    aria_describedby,
                }
                ChevronDown {
                    size: "1rem",
                    class: "pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-muted transition-transform \
                            group-has-[[aria-expanded=true]]:rotate-180",
                }
            }
            combobox::ComboboxList { class: "absolute top-full left-0 z-50 mt-1 max-h-80 min-w-full overflow-auto border border-control \
                        bg-surface py-1 text-ink shadow-popover \
                        data-[state=open]:animate-popover-in data-[state=closed]:animate-popover-out",
                {children}
            }
        }
    }
}

#[component]
pub fn ComboboxEmpty(props: ComboboxEmptyProps) -> Element {
    let base = attributes!(div { class: "px-3 py-2 text-body text-muted" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        combobox::ComboboxEmpty { attributes: merged, {props.children} }
    }
}

#[component]
pub fn ComboboxOption<T: Clone + PartialEq + 'static>(props: ComboboxOptionProps<T>) -> Element {
    let base = attributes!(div {
        class: "flex min-h-9 cursor-pointer items-center justify-between gap-2 px-3 py-2 text-body outline-none \
                select-none aria-selected:bg-subtle aria-selected:font-medium \
                data-[highlighted=true]:bg-accent data-[highlighted=true]:text-accent-ink \
                data-[disabled=true]:pointer-events-none data-[disabled=true]:text-muted",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        combobox::ComboboxOption::<T> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            aria_label: props.aria_label,
            aria_roledescription: props.aria_roledescription,
            attributes: merged,
            {props.children}
            combobox::ComboboxItemIndicator {
                Check { size: "1rem", class: "shrink-0" }
            }
        }
    }
}
