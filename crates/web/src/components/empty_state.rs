use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

/// What to do when a view has nothing to show.
///
/// Keep empty, loading, offline, and failed as different states. The children are
/// the way out, such as "Add a movie or series" or "Clear filters".
///
/// ```ignore
/// view! {
///     empty_state(
///         title: "No files need review.",
///         description: "New downloads that cannot be matched appear here."
///     )
/// }
/// ```
#[component]
pub async fn empty_state(
    title: &str,
    #[into]
    #[default]
    description: Option<&str>,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("flex flex-col items-start gap-3 py-10", attrs.remove("class")))
            (attrs)
        >
            <p class="text-section font-medium">(title)</p>
            <p class="max-w-prose text-muted empty:hidden">(description)</p>
            <div class="flex flex-wrap gap-2 empty:hidden">(child)</div>
        </div>
    })
}
