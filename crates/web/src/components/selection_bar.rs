use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

/// The toolbar shown once rows are selected: the selection summary, then the actions
/// valid for that selection.
///
/// `summary` states the count, such as "3 of 12 files selected". Pass fields and
/// buttons as children; fields show "Mixed" rather than the first row's value when
/// the selection disagrees.
///
/// ```ignore
/// view! {
///     selection_bar(
///         summary: "3 files selected",
///         button("Detect again")
///     )
/// }
/// ```
#[component]
pub async fn selection_bar(
    summary: &str,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="toolbar"
            aria-label="Selected files"
            class=(class!("flex flex-wrap items-end gap-2 border-b border-line pb-3", attrs.remove("class")))
            (attrs)
        >
            <span class="mr-auto self-center text-caption text-muted" aria-live="polite">(summary)</span>
            (child)
        </div>
    })
}
