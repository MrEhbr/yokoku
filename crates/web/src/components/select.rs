use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// Classes for the select control, with space for a custom dropdown arrow.
const SELECT: StaticClass = class!(
    "min-h-9 w-full appearance-none items-center border border-control \
     bg-surface py-1.5 pr-8 pl-2 text-left text-body text-ink transition-colors \
     aria-invalid:border-danger aria-invalid:focus-visible:outline-danger \
     disabled:cursor-not-allowed disabled:bg-subtle disabled:text-muted",
);

/// Classes for browsers that support customizable select pickers. Other browsers use
/// their native picker.
const PICKER: StaticClass = class!(
    "[&::picker(select)]:[appearance:base-select] \
     [&::picker(select)]:my-1 [&::picker(select)]:border \
     [&::picker(select)]:border-control [&::picker(select)]:bg-surface \
     [&::picker(select)]:text-ink [&::picker(select)]:shadow-popover \
     [&::picker(select)]:min-w-[anchor-size(width)] \
     [&::picker(select)]:max-w-[calc(100dvw-2rem)] \
     [&::picker(select)]:max-h-[min(20rem,60dvh)] [&::picker(select)]:overflow-auto \
     [&::picker-icon]:hidden \
     [&_optgroup>legend]:px-3 [&_optgroup>legend]:py-1.5 \
     [&_optgroup>legend]:text-caption [&_optgroup>legend]:font-medium \
     [&_optgroup>legend]:text-muted [&_optgroup>legend]:cursor-default \
     [&_optgroup>legend]:select-none \
     [&_option]:flex [&_option]:min-h-9 [&_option]:items-center [&_option]:gap-2 \
     [&_option]:px-3 [&_option]:py-2 [&_option]:text-body \
     [&_option]:whitespace-normal [&_option]:wrap-anywhere \
     [&_option]:outline-offset-[-2px] \
     [&_option:checked]:bg-subtle [&_option:checked]:font-medium \
     [&_option:not(:disabled):is(:hover,:focus-visible)]:bg-accent \
     [&_option:not(:disabled):is(:hover,:focus-visible)]:text-accent-ink \
     [&_option:disabled]:text-muted \
     [&_option::checkmark]:order-1 [&_option::checkmark]:ml-auto \
     [&_option::checkmark]:size-4 [&_option::checkmark]:shrink-0 \
     [&_option::checkmark]:content-[''] [&_option::checkmark]:bg-current \
     [&_option::checkmark]:[mask-size:100%_100%] \
     [&_option::checkmark]:[mask-image:var(--select-checkmark)]",
);

/// The icon marking the picker's checked option.
const CHECKMARK: IconData = iconify_icon!("lucide:check");

/// Supplies the checkmark icon as a data URI in `--select-checkmark` so CSS can use it
/// as a mask.
fn checkmark_style(cx: &Cx) -> String {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{}">{}</svg>"#,
        CHECKMARK.view_box(),
        CHECKMARK.into_body().render(cx),
    );
    let mut style = String::from(r#"--select-checkmark: url("data:image/svg+xml,"#);
    // Percent-encode the characters that cannot appear in a double-quoted
    // CSS url().
    for char in svg.chars() {
        match char {
            '%' => style.push_str("%25"),
            '"' => style.push_str("%22"),
            '#' => style.push_str("%23"),
            _ => style.push(char),
        }
    }
    style.push_str(r#"")"#);
    style
}

/// A styled native select control.
///
/// Pass `<option>` or `<optgroup>` elements as children. Classes in `attrs` apply to
/// the wrapper, while other attributes and event handlers go on the `<select>`. The
/// control fills its container by default. Set `aria-invalid="true"` to show the error
/// border and focus ring.
///
/// Browsers with customizable select support also style the picker. For a styled group
/// heading, place a `<legend>` first inside an `<optgroup>` and keep its `label`
/// attribute for browsers that use the native picker.
///
/// ```ignore
/// view! {
///     select(
///         attrs: attributes! { name="region" },
///         <option>"eu-central-1"</option>
///         <option>"us-east-1"</option>
///     )
/// }
/// ```
#[component]
pub async fn select(cx: &Cx, #[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    // `appearance: base-select` opts into the customizable picker. It is set
    // from the wrapper because the descendant selector outranks the
    // `appearance-none` fallback in specificity, making the outcome
    // independent of stylesheet order; browsers without support drop the
    // invalid declaration and keep the fallback.
    Ok(view! {
        <span
            class=(class!(
                "relative block \
                 [&>select]:[appearance:base-select] \
                 [&:has(select:open)>svg]:rotate-180",
                attrs.remove("class"),
            ))
            style=(checkmark_style(cx))
        >
            <select class=(class!(SELECT, PICKER)) (attrs)>(child)</select>
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="pointer-events-none absolute top-1/2 right-2.5 size-4 \
                        -translate-y-1/2 text-muted transition-transform"
                }
            )
        </span>
    })
}
