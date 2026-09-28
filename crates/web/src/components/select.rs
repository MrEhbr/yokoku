use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, ChevronDown};
pub use dioxus_primitives::select::SelectGroup;
use dioxus_primitives::{
    dioxus_attributes::attributes,
    merge_attributes,
    select::{self, SelectGroupLabelProps, SelectOptionProps},
};

/// Stops Escape pressed in a listbox at `<body>`, after Dioxus closed the list at the app root,
/// so a dialog's document-level Escape listener doesn't close the dialog around the select.
/// Dioxus has already marked the list closed by then; focus is in a listbox only while it's open.
const CONTAIN_ESCAPE: &str = r#"if (!window.ykSelectEscape) {
  window.ykSelectEscape = true;
  document.body.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && event.target.closest?.('[role="listbox"]')) {
      event.stopPropagation();
    }
  });
}"#;

/// A single-choice picker over typed values: `on_value_change` receives the chosen `T`.
///
/// Controlled with `value`, or uncontrolled with `default_value`. `placeholder` shows while
/// nothing is selected, and also until the options register on the client, so a server-rendered
/// select with a value passes that value's text as `placeholder`. Give it an `aria_label` when no
/// visible label names it.
#[component]
pub fn Select<T: Clone + PartialEq + 'static>(
    #[props(default)] value: Option<ReadSignal<Option<T>>>,
    #[props(default)] default_value: Option<T>,
    #[props(default)] on_value_change: Callback<Option<T>>,
    #[props(default)] disabled: ReadSignal<bool>,
    #[props(default)] name: ReadSignal<String>,
    #[props(into, default = "Select…".to_owned())] placeholder: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "group relative block w-full" });
    let merged = merge_attributes(vec![base, attributes]);
    use_effect(|| _ = document::eval(CONTAIN_ESCAPE));

    rsx! {
        select::Select {
            value,
            default_value,
            on_value_change,
            disabled,
            name,
            attributes: merged,
            select::SelectTrigger { class: "flex min-h-9 w-full cursor-pointer items-center justify-between gap-2 border border-control \
                        bg-surface py-1.5 pr-2 pl-2 text-left text-body text-ink transition-colors \
                        group-data-[disabled=true]:cursor-not-allowed group-data-[disabled=true]:bg-subtle \
                        group-data-[disabled=true]:text-muted",
                select::SelectValue {
                    class: "truncate data-[placeholder=true]:text-muted",
                    placeholder,
                }
                ChevronDown {
                    size: "1rem",
                    class: "shrink-0 text-muted transition-transform group-data-[state=open]:rotate-180",
                }
            }
            select::SelectList { class: "absolute top-full left-0 z-50 mt-1 max-h-80 min-w-full overflow-auto border border-control \
                        bg-surface py-1 text-ink shadow-popover \
                        data-[state=open]:animate-popover-in data-[state=closed]:animate-popover-out",
                {children}
            }
        }
    }
}

#[component]
pub fn SelectGroupLabel(props: SelectGroupLabelProps) -> Element {
    let base = attributes!(div { class: "cursor-default px-3 py-1.5 text-caption font-medium text-muted select-none" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        select::SelectGroupLabel { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn SelectOption<T: Clone + PartialEq + 'static>(props: SelectOptionProps<T>) -> Element {
    let base = attributes!(div {
        class: "flex min-h-9 cursor-pointer items-center justify-between gap-2 px-3 py-2 text-body outline-none \
                select-none aria-selected:bg-subtle aria-selected:font-medium \
                hover:bg-accent hover:text-accent-ink focus:bg-accent focus:text-accent-ink \
                data-[disabled=true]:pointer-events-none data-[disabled=true]:text-muted",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        select::SelectOption::<T> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            aria_label: props.aria_label,
            aria_roledescription: props.aria_roledescription,
            attributes: merged,
            {props.children}
            select::SelectItemIndicator {
                Check { size: "1rem", class: "shrink-0" }
            }
        }
    }
}
