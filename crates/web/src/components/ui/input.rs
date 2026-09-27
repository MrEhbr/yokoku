use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// Classes for the input's dimensions, border, and interaction states.
const INPUT: StaticClass = class!(
    "min-h-9 w-full min-w-0 border border-control bg-surface px-2 py-1.5 \
     text-body text-ink transition-colors \
     placeholder:text-muted \
     file:mr-3 file:h-full file:border-0 file:bg-transparent file:text-body file:font-medium \
     aria-invalid:border-danger aria-invalid:focus-visible:outline-danger \
     disabled:cursor-not-allowed disabled:bg-subtle disabled:text-muted",
);

/// A styled input.
///
/// Pass input attributes and event handlers through `attrs`. Extra classes are added to
/// the input's classes. It fills its container by default. Set `aria-invalid="true"` to
/// show the error border and focus ring.
///
/// ```ignore
/// view! {
///     input(attrs: attributes! { type="email" placeholder="you@example.com" })
/// }
/// ```
#[component]
pub async fn input(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! { <input class=(class!(INPUT, attrs.remove("class"))) (attrs)> })
}
