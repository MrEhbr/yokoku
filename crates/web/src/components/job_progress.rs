use topcoat::{
    Result,
    view::{Attributes, Child, View, attributes, class, component, view},
};

use crate::components::ui::progress::progress;

/// A long operation: its name, the numbers so far, and a progress bar.
///
/// `detail` carries percent, speed, and time left, such as "64% · 4.1 MB/s · 2 min".
/// Omit `value` while the amount is unknown; the bar is then indeterminate, not 0%.
/// Children follow the bar: a failure reason with a retry action, for example.
///
/// ```ignore
/// view! {
///     job_progress(title: "Orbital · Season 1", detail: "64% · 2 min left", value: 64.0)
/// }
/// ```
#[component]
pub async fn job_progress(
    title: &str,
    #[into]
    #[default]
    detail: Option<&str>,
    #[into]
    #[default]
    value: Option<f32>,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("grid gap-1.5", attrs.remove("class"))) (attrs)>
            <div class="flex flex-wrap justify-between gap-x-3 text-caption">
                <span class="font-medium">(title)</span>
                <span class="text-muted tabular-nums">(detail)</span>
            </div>
            progress(value: value, attrs: attributes! { aria-label=(title) })
            <div class="empty:hidden">(child)</div>
        </div>
    })
}
