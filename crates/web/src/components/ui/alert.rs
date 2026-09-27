use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The visual style of an [`alert`].
///
/// [`Default`] is `AlertVariant::Neutral`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AlertVariant {
    /// A plain notice.
    #[default]
    Neutral,
    /// An informational or upcoming state.
    Info,
    /// Something that needs review.
    Warning,
    /// A failure, with the reason and a way to retry.
    Danger,
}

impl AlertVariant {
    /// Classes for the variant's rule, fill, and text colors.
    fn classes(self) -> StaticClass {
        match self {
            Self::Neutral => class!("border-control bg-subtle text-ink"),
            Self::Info => class!("border-info bg-info-soft text-info"),
            Self::Warning => class!("border-warning bg-warning-soft text-warning"),
            Self::Danger => class!("border-danger bg-danger-soft text-danger"),
        }
    }
}

/// Classes for the alert layout. The icon column collapses when no icon is present.
const BASE: StaticClass = class!(
    "grid w-full grid-cols-[0_1fr] items-start gap-y-0.5 border-l-2 \
     px-3 py-2 text-body has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-2.5 \
     [&>svg]:size-4 [&>svg]:translate-y-0.5",
);

/// A notice displayed within the page.
///
/// Use `variant` to choose its style. Pass an optional icon, an `alert_title`, and an
/// `alert_description` as children. `attrs` are forwarded to the `<div>`, with extra
/// classes added to its classes.
///
/// ```ignore
/// view! {
///     alert(
///         variant: AlertVariant::Danger,
///         icon(data: iconify_icon!("lucide:triangle-alert"))
///         alert_title("Build failed")
///         alert_description("The last deploy did not finish.")
///     )
/// }
/// ```
#[component]
pub async fn alert(
    /// The visual style of the notice.
    #[default]
    variant: AlertVariant,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The alert's icon, title, and description.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    // `role="alert"` is deliberately absent: it interrupts a screen reader
    // the moment the element appears, which suits a message arriving during
    // the visit, not one rendered with the page. Pass it among the `attrs`
    // where that is what you want.
    Ok(view! {
        <div class=(class!(BASE, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The heading of an alert.
#[component]
pub async fn alert_title(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <p
            class=(class!(
                "col-start-2 font-medium tracking-tight",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </p>
    })
}

/// Text that explains the alert and any action the reader should take.
#[component]
pub async fn alert_description(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "col-start-2 text-body",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
