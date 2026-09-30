use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// A text input for multiple lines. Grows with its content where the browser supports
/// content sizing. Set `aria_invalid: "true"` for the error border and focus ring.
#[component]
pub fn Textarea(
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] onchange: EventHandler<FormEvent>,
    #[props(default)] oninvalid: EventHandler<FormEvent>,
    #[props(default)] onselect: EventHandler<SelectionEvent>,
    #[props(default)] onselectionchange: EventHandler<SelectionEvent>,
    #[props(default)] onfocus: EventHandler<FocusEvent>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onfocusin: EventHandler<FocusEvent>,
    #[props(default)] onfocusout: EventHandler<FocusEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
    #[props(default)] onkeypress: EventHandler<KeyboardEvent>,
    #[props(default)] onkeyup: EventHandler<KeyboardEvent>,
    #[props(default)] oncompositionstart: EventHandler<CompositionEvent>,
    #[props(default)] oncompositionupdate: EventHandler<CompositionEvent>,
    #[props(default)] oncompositionend: EventHandler<CompositionEvent>,
    #[props(default)] oncopy: EventHandler<ClipboardEvent>,
    #[props(default)] oncut: EventHandler<ClipboardEvent>,
    #[props(default)] onpaste: EventHandler<ClipboardEvent>,
    #[props(default)] onmounted: EventHandler<MountedEvent>,
    #[props(extends=GlobalAttributes)]
    #[props(extends=textarea)]
    attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(textarea {
        "data-slot": "textarea",
        class: "field-sizing-content min-h-16 w-full border border-control bg-surface px-2 py-1.5 \
                text-body text-ink transition-colors placeholder:text-muted aria-invalid:border-danger \
                disabled:cursor-not-allowed disabled:bg-subtle disabled:text-muted",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        textarea {
            oninput: move |e| oninput.call(e),
            onchange: move |e| onchange.call(e),
            oninvalid: move |e| oninvalid.call(e),
            onselect: move |e| onselect.call(e),
            onselectionchange: move |e| onselectionchange.call(e),
            onfocus: move |e| onfocus.call(e),
            onblur: move |e| onblur.call(e),
            onfocusin: move |e| onfocusin.call(e),
            onfocusout: move |e| onfocusout.call(e),
            onkeydown: move |e| onkeydown.call(e),
            onkeypress: move |e| onkeypress.call(e),
            onkeyup: move |e| onkeyup.call(e),
            oncompositionstart: move |e| oncompositionstart.call(e),
            oncompositionupdate: move |e| oncompositionupdate.call(e),
            oncompositionend: move |e| oncompositionend.call(e),
            oncopy: move |e| oncopy.call(e),
            oncut: move |e| oncut.call(e),
            onpaste: move |e| onpaste.call(e),
            onmounted: move |e| onmounted.call(e),
            ..merged,
        }
    }
}
