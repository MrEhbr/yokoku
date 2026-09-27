use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// Classes for the native progress track and fill. Indeterminate animation depends on
/// the browser and may appear as an empty track.
const PROGRESS: StaticClass = class!(
    "h-1.5 w-full appearance-none overflow-hidden \
     bg-subtle [&::-webkit-progress-bar]:bg-transparent \
     [&::-webkit-progress-value]:bg-ink \
     [&::-webkit-progress-value]:transition-all \
     [&::-moz-progress-bar]:bg-ink [&:indeterminate::-moz-progress-bar]:bg-transparent \
     indeterminate:bg-linear-to-r indeterminate:from-ink indeterminate:to-ink \
     indeterminate:bg-size-[30%_100%] indeterminate:bg-no-repeat \
     indeterminate:animate-progress-indeterminate \
     motion-reduce:indeterminate:animate-none motion-reduce:indeterminate:bg-size-[auto] \
     motion-reduce:indeterminate:bg-[repeating-linear-gradient(90deg,var(--yk-ink)_0_6px,transparent_6px_12px)]",
);

/// A native progress bar.
///
/// `value` is the completed amount out of `max`, which defaults to 100. Omit `value`
/// when the total work is unknown. The indeterminate appearance depends on the browser.
///
/// `attrs` are forwarded to the `<progress>`, with extra classes added to its classes.
/// Give it an accessible label through `aria-label` or an associated label element. The
/// bar fills its container by default.
///
/// ```ignore
/// view! {
///     progress(value: 62.0)
/// }
/// ```
#[component]
pub async fn progress(
    /// The completed amount, out of `max`. `None` renders an indeterminate bar.
    #[into]
    #[default]
    value: Option<f32>,
    /// The amount that counts as complete.
    #[default(100.0)]
    max: f32,
    /// Extra attributes for the `<progress>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <progress
            class=(class!(PROGRESS, attrs.remove("class")))
            value=(value)
            max=(max)
            (attrs)
        ></progress>
    })
}
