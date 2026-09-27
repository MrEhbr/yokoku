use topcoat::{
    Result,
    view::{Attributes, Child, Class, StaticClass, View, class, component, view},
};

/// The visual style of a [`button`].
///
/// [`Default`] is `ButtonVariant::Secondary`, used when no variant is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonVariant {
    /// The pink button for the one primary action in a local context.
    Primary,
    /// A control-bordered button for every other action.
    #[default]
    Secondary,
    /// No border until hovered, for toolbars and inline actions.
    Quiet,
    /// A danger-bordered button for destructive actions. Never pink.
    Danger,
}

impl ButtonVariant {
    /// Classes for the button variant and its interaction states.
    ///
    /// Each variant sets its own border color. Keep border colors out of the shared
    /// base to avoid conflicting classes.
    fn classes(self) -> StaticClass {
        match self {
            Self::Primary => class!("border-ink bg-accent text-accent-ink hover:not-disabled:bg-accent-hover",),
            Self::Secondary => class!("border-control text-ink hover:not-disabled:bg-subtle"),
            Self::Quiet => class!(
                "border-transparent text-muted hover:not-disabled:bg-subtle \
                 hover:not-disabled:text-ink",
            ),
            Self::Danger => class!("border-danger text-danger hover:not-disabled:bg-danger-soft"),
        }
    }
}

/// The size of a [`button`].
///
/// [`Default`] is `ButtonSize::Md`, used when no size is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonSize {
    /// A compact button for dense rows.
    Sm,
    /// The standard 36px control height.
    #[default]
    Md,
    /// A square button sized for a single icon. Give it an `aria-label`.
    Icon,
}

impl ButtonSize {
    /// Classes for the button dimensions. Text size stays the same across sizes.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("min-h-8 gap-1.5 px-2.5 py-1"),
            Self::Md => class!("min-h-9 gap-2 px-3 py-1.5"),
            Self::Icon => class!("size-9 [&>svg]:size-4"),
        }
    }
}

/// Classes shared by button variants and sizes. A border reserves the same space in
/// every variant.
const BASE: StaticClass = class!(
    "inline-flex shrink-0 cursor-pointer items-center justify-center border \
     text-body font-medium whitespace-nowrap select-none \
     transition-colors duration-120 ease-interface motion-reduce:transition-none \
     disabled:cursor-not-allowed disabled:opacity-45 aria-busy:cursor-progress",
);

/// Builds the full class list for a button of the given `variant` and `size`.
///
/// Use it to give button styling to an element that is not a `<button>`, such
/// as a link styled as a button:
///
/// ```ignore
/// view! {
///     <a href="/login" class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))>
///         "Sign in"
///     </a>
/// }
/// ```
#[must_use]
pub fn button_variants(variant: ButtonVariant, size: ButtonSize) -> Class<(StaticClass, StaticClass, StaticClass)> {
    class!(BASE, variant.classes(), size.classes())
}

/// A styled button.
///
/// `variant` defaults to `Secondary` and `size` to `Md`. Pass the content as children.
/// `attrs` are forwarded to the `<button>`, with extra classes added to its classes.
/// Use [`button_variants`] to apply the same styling to another element.
///
/// ```ignore
/// view! {
///     button(
///         variant: ButtonVariant::Danger,
///         attrs: attributes! { type="submit" },
///         "Delete"
///     )
/// }
/// ```
#[component]
pub async fn button(
    #[default] variant: ButtonVariant,
    #[default] size: ButtonSize,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <button
            class=(class!(
                BASE,
                variant.classes(),
                size.classes(),
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </button>
    })
}
