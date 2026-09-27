use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, StaticClass, View, attributes, class, component, view},
};

/// Classes for the native checkbox input and its checked and indeterminate states.
const CHECKBOX: StaticClass = class!(
    "peer size-4 shrink-0 cursor-pointer appearance-none border border-control \
     bg-surface transition-colors \
     checked:border-ink checked:bg-ink \
     indeterminate:border-ink indeterminate:bg-ink \
     disabled:cursor-not-allowed",
);

/// Classes shared by the overlaid check and dash marks.
const MARK: &str = "pointer-events-none absolute inset-0 m-auto size-3.5 text-canvas opacity-0";

/// A styled native checkbox.
///
/// Pass input attributes and event handlers through `attrs`. Classes apply to the
/// wrapper, while other attributes go on the `<input>`. Use `checked` for the initial
/// state. The indeterminate state is a DOM property, set from script; it shows a dash.
///
/// ```ignore
/// view! {
///     <div class="flex items-center gap-2">
///         checkbox(attrs: attributes! { id="terms" name="terms" checked="" })
///         label(attrs: attributes! { for="terms" }, "Accept terms")
///     </div>
/// }
/// ```
#[component]
pub async fn checkbox(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The checkmark cannot be drawn by the `<input>` itself, which renders no
    // children or pseudo-elements: it is a sibling icon overlaid on the
    // control, revealed by the input's `peer` state while checked.
    Ok(view! {
        <span
            class=(class!(
                "peer relative inline-flex shrink-0 has-[:disabled]:opacity-45",
                attrs.remove("class"),
            ))
        >
            <input type="checkbox" class=(CHECKBOX) (attrs)>
            icon(
                data: iconify_icon!("lucide:check"),
                attrs: attributes! {
                    class=(class!(MARK, "peer-[:checked:not(:indeterminate)]:opacity-100"))
                }
            )
            icon(
                data: iconify_icon!("lucide:minus"),
                attrs: attributes! { class=(class!(MARK, "peer-indeterminate:opacity-100")) }
            )
        </span>
    })
}
