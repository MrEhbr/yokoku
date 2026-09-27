use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, Child, View, class, component, view},
};

use crate::components::checkbox::checkbox;

/// One file in an import review or rename preview: a checkbox with the current name,
/// an arrow, and the new name.
///
/// Pass the new side as children: a `yk-code` span for a fixed destination, or a
/// [`select`](crate::components::select::select) of episode identities during import
/// review. `checkbox_attrs` go on the selection checkbox; give it `name`, `value`,
/// and `checked`. Below `sm` the old name takes its own line.
///
/// ```ignore
/// view! {
///     file_row(
///         old: "Orbital.S01E01.mkv",
///         checkbox_attrs: attributes! { name="file" value=(id) checked="" },
///         <span class="yk-code break-all">"Orbital (2024) - S01E01 - Launch.mkv"</span>
///     )
/// }
/// ```
#[component]
pub async fn file_row(
    cx: &Cx,
    /// The file's current name.
    old: &str,
    /// Attributes for the selection checkbox.
    #[default]
    mut checkbox_attrs: Attributes,
    /// Extra attributes for the row.
    #[default]
    mut attrs: Attributes,
    /// The new name or the control that chooses it.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    checkbox_attrs.insert(cx, "aria-label", format!("Select {old}"));
    Ok(view! {
        <div
            class=(class!(
                "grid grid-cols-[1rem_minmax(0,1fr)] items-center gap-2 border-t border-line py-3 first:border-t-0 \
                 sm:grid-cols-[minmax(0,1fr)_1rem_minmax(0,1.65fr)] sm:gap-3",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <label class="col-span-full flex min-w-0 cursor-pointer items-center gap-2 sm:col-span-1">
                checkbox(attrs: checkbox_attrs)
                <span class="yk-code break-all">(old)</span>
            </label>
            <span class="text-muted" aria-hidden="true">"→"</span>
            <div class="min-w-0">(child)</div>
        </div>
    })
}
