use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Classes that align label content and reflect the disabled state of a nearby or
/// nested control.
const LABEL: StaticClass = class!(
    "flex items-center gap-2 leading-none font-medium select-none \
     peer-disabled:pointer-events-none peer-disabled:opacity-45 \
     peer-has-[:disabled]:pointer-events-none peer-has-[:disabled]:opacity-45 \
     has-[+:disabled]:pointer-events-none has-[+:disabled]:opacity-45 \
     has-[:is(input,select,textarea,button):disabled]:pointer-events-none \
     has-[:is(input,select,textarea,button):disabled]:opacity-45",
);

/// A label for a form control.
///
/// Wrap the control or pass a `for` attribute matching its `id`. Pass the label text as
/// children. `attrs` are forwarded to the `<label>`, with extra classes added to its
/// classes.
///
/// ```ignore
/// view! {
///     <div class="flex flex-col gap-2">
///         label(attrs: attributes! { for="email" }, "Email")
///         input(attrs: attributes! { id="email" type="email" })
///     </div>
/// }
/// ```
#[component]
pub async fn label(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <label class=(class!(LABEL, attrs.remove("class"))) (attrs)>(child)</label>
    })
}
