use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

/// A page's `<h1>` with the page-level actions beside it.
///
/// ```ignore
/// view! {
///     page_header(title: "Library", button(variant: ButtonVariant::Primary, "Add series"))
/// }
/// ```
#[component]
pub async fn page_header(
    title: &str,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <header
            class=(class!("flex flex-wrap items-end justify-between gap-4 py-8", attrs.remove("class")))
            (attrs)
        >
            <h1 class="yk-page-title">(title)</h1>
            <div class="flex flex-wrap items-center gap-2">(child)</div>
        </header>
    })
}
