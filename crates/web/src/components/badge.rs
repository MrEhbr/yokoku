use topcoat::{
    Result,
    view::{Attributes, Child, Class, StaticClass, View, class, component, view},
};

/// The visual style of a [`badge`].
///
/// Pair a status variant with a label and an icon.
///
/// [`Default`] is `BadgeVariant::Neutral`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BadgeVariant {
    /// A quiet tag for neutral labels and counts.
    #[default]
    Neutral,
    /// A bordered tag on the page background.
    Outline,
    /// A present file or a completed operation.
    Success,
    /// A missing file or something that needs review.
    Warning,
    /// A failure or conflict.
    Danger,
    /// An upcoming or informational state.
    Info,
}

impl BadgeVariant {
    /// Classes for the variant, including its border color. Keep border colors out of
    /// the shared base to avoid conflicting classes.
    fn classes(self) -> StaticClass {
        match self {
            Self::Neutral => class!("border-line bg-subtle text-ink"),
            Self::Outline => class!("border-control text-ink"),
            Self::Success => class!("border-transparent bg-success-soft text-success"),
            Self::Warning => class!("border-transparent bg-warning-soft text-warning"),
            Self::Danger => class!("border-transparent bg-danger-soft text-danger"),
            Self::Info => class!("border-transparent bg-info-soft text-info"),
        }
    }
}

/// Classes shared by badge variants. A border reserves the same space in every variant.
const BASE: StaticClass = class!(
    "inline-flex w-fit shrink-0 items-center justify-center gap-1 \
     border px-2 py-0.5 text-caption font-medium whitespace-nowrap [&>svg]:size-3",
);

/// Builds the full class list for a badge of the given `variant`.
///
/// Use it to give badge styling to another element, such as a link:
///
/// ```ignore
/// view! {
///     <a href="/releases/v2" class=(badge_variants(BadgeVariant::Outline))>"v2.0"</a>
/// }
/// ```
#[must_use]
pub fn badge_variants(variant: BadgeVariant) -> Class<(StaticClass, StaticClass)> {
    class!(BASE, variant.classes())
}

/// A small label for a status or count.
///
/// `variant` defaults to `Neutral`. Pass the label as children and extra attributes
/// through `attrs`. Attributes go on the `<span>`, with classes added to its classes.
/// Use [`badge_variants`] to apply the same styling to another element.
///
/// ```ignore
/// view! {
///     badge(variant: BadgeVariant::Danger, "Failed")
/// }
/// ```
#[component]
pub async fn badge(
    #[default] variant: BadgeVariant,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span class=(class!(BASE, variant.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </span>
    })
}
