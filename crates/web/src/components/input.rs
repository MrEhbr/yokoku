use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Input(
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
    #[props(default)] onwheel: EventHandler<WheelEvent>,
    #[props(default)] oncompositionstart: EventHandler<CompositionEvent>,
    #[props(default)] oncompositionupdate: EventHandler<CompositionEvent>,
    #[props(default)] oncompositionend: EventHandler<CompositionEvent>,
    #[props(default)] oncopy: EventHandler<ClipboardEvent>,
    #[props(default)] oncut: EventHandler<ClipboardEvent>,
    #[props(default)] onpaste: EventHandler<ClipboardEvent>,
    #[props(extends=GlobalAttributes)]
    #[props(extends=input)]
    attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(input {
        class: "min-h-9 w-full min-w-0 border border-control bg-surface px-2 py-1.5 text-body text-ink \
                placeholder:text-muted aria-invalid:border-danger disabled:cursor-not-allowed \
                disabled:bg-subtle disabled:text-muted",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        input {
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
            onwheel: move |e| onwheel.call(e),
            oncompositionstart: move |e| oncompositionstart.call(e),
            oncompositionupdate: move |e| oncompositionupdate.call(e),
            oncompositionend: move |e| oncompositionend.call(e),
            oncopy: move |e| oncopy.call(e),
            oncut: move |e| oncut.call(e),
            onpaste: move |e| onpaste.call(e),
            ..merged,
        }
    }
}
