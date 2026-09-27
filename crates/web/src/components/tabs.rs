use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A group of panels with controls for selecting the visible panel.
///
/// Triggers can navigate to a server-rendered panel or update a signal. For browser
/// updates, bind each trigger's `active` prop and each panel's `hidden` attribute to
/// the selected-tab signal. Triggers are ordinary links and do not implement the ARIA
/// tab pattern's arrow-key navigation.
///
/// `attrs` are forwarded to the outer `<div>`, with extra classes added to its classes.
///
/// ```ignore
/// view! {
///     tabs(
///         tabs_list(
///             for (value, text) in TABS {
///                 tabs_trigger(
///                     active: value == tab,
///                     attrs: attributes! { href=(format!("?tab={value}")) },
///                     (text)
///                 )
///             }
///         )
///         tabs_content((panel))
///     )
/// }
/// ```
#[component]
pub async fn tabs(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("flex flex-col gap-4", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// A row of controls for selecting a panel.
#[component]
pub async fn tabs_list(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex flex-wrap border-b border-line",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// Classes for a tab trigger's active and hover states.
const TRIGGER: StaticClass = class!(
    "-mb-px inline-flex min-h-9 shrink-0 cursor-pointer items-center justify-center gap-2 \
     border-b-2 border-transparent px-3 py-2 text-body whitespace-nowrap text-muted \
     transition-colors hover:bg-subtle hover:text-ink \
     aria-[current=page]:border-ink aria-[current=page]:text-ink",
);

/// A link that selects a panel.
///
/// `active` accepts a boolean or runtime expression and controls the selected styling
/// and `aria-current="page"`. Pass the destination as `href` in `attrs`. To select a
/// panel locally, handle the click, prevent navigation, and update the selected-tab
/// signal.
#[component]
pub async fn tabs_trigger(
    /// Whether this trigger selects the visible panel.
    #[into]
    #[default(false.into())]
    active: Expr<bool>,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The trigger's label.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <a
            :aria-current=$(active.then_some("page"))
            class=(class!(TRIGGER, attrs.remove("class")))
            (attrs)
        >
            (child)
        </a>
    })
}

/// A panel selected by a tab trigger.
///
/// Render only the selected panel on the server, or render all panels with `:hidden`
/// bindings to switch between them in the browser.
#[component]
pub async fn tabs_content(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class=(attrs.remove("class")) (attrs)>(child)</div> })
}
