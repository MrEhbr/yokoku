use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Classes for a textarea that grows with its content. Browsers without content sizing
/// support keep the minimum height and scroll.
const TEXTAREA: StaticClass = class!(
    "field-sizing-content min-h-16 w-full border border-control \
     bg-surface px-2 py-1.5 text-body text-ink transition-colors \
     placeholder:text-muted \
     aria-invalid:border-danger aria-invalid:focus-visible:outline-danger \
     disabled:cursor-not-allowed disabled:bg-subtle disabled:text-muted",
);

/// A text input for multiple lines.
///
/// Pass the initial value as children. `attrs` are forwarded to the `<textarea>`, with
/// extra classes added to its classes. It fills its container and grows with its
/// content where the browser supports this. Set `aria-invalid="true"` to show the error
/// border and focus ring.
///
/// ```ignore
/// view! {
///     textarea(attrs: attributes! { name="feedback" placeholder="Tell us more" })
/// }
/// ```
#[component]
pub async fn textarea(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <textarea class=(class!(TEXTAREA, attrs.remove("class"))) (attrs)>
            (child)
        </textarea>
    })
}
