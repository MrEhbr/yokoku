use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[derive(Copy, Clone, PartialEq, Default)]
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
    /// Each variant sets its own border color; the shared base sets none.
    fn class(self) -> &'static str {
        match self {
            Self::Primary => "border-ink bg-accent text-accent-ink hover:not-disabled:bg-accent-hover",
            Self::Secondary => "border-control text-ink hover:not-disabled:bg-subtle",
            Self::Quiet => "border-transparent text-muted hover:not-disabled:bg-subtle hover:not-disabled:text-ink",
            Self::Danger => "border-danger text-danger hover:not-disabled:bg-danger-soft",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    /// A square button holding one icon; give it an `aria_label`.
    Icon,
}

impl ButtonSize {
    fn class(self) -> &'static str {
        match self {
            Self::Sm => "min-h-8 gap-1.5 px-2.5 py-1",
            Self::Md => "min-h-9 gap-2 px-3 py-1.5",
            Self::Icon => "size-9 [&>svg]:size-4",
        }
    }
}

#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(extends=GlobalAttributes)]
    #[props(extends=button)]
    attributes: Vec<Attribute>,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(default)] onmousedown: EventHandler<MouseEvent>,
    #[props(default)] onmouseup: EventHandler<MouseEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
    children: Element,
) -> Element {
    let (variant, size) = (variant.class(), size.class());
    let base = attributes!(button {
        r#type: "button",
        class: "inline-flex shrink-0 cursor-pointer items-center justify-center border text-body font-medium \
                whitespace-nowrap select-none transition-colors duration-120 ease-interface \
                motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-45 \
                aria-busy:cursor-progress {variant} {size}",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        button {
            onclick: move |event| onclick.call(event),
            onmousedown: move |event| onmousedown.call(event),
            onmouseup: move |event| onmouseup.call(event),
            onkeydown: move |event| onkeydown.call(event),
            ..merged,
            {children}
        }
    }
}
