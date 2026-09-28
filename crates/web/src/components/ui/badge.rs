use dioxus::prelude::*;

#[derive(Copy, Clone, PartialEq, Default)]
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
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => "border-line bg-subtle text-ink",
            Self::Outline => "border-control text-ink",
            Self::Success => "border-transparent bg-success-soft text-success",
            Self::Warning => "border-transparent bg-warning-soft text-warning",
            Self::Danger => "border-transparent bg-danger-soft text-danger",
            Self::Info => "border-transparent bg-info-soft text-info",
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct BadgeProps {
    #[props(default)]
    pub variant: BadgeVariant,

    /// Additional attributes to extend the badge element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the badge element
    pub children: Element,
}

/// A small label. Pair a status color with words; color alone never carries the meaning.
#[component]
pub fn Badge(props: BadgeProps) -> Element {
    let variant = props.variant.class();
    rsx! {
        span {
            class: "inline-flex w-fit shrink-0 items-center justify-center gap-1 border px-2 py-0.5 text-caption \
                    font-medium whitespace-nowrap [&>svg]:size-3 {variant}",
            ..props.attributes,
            {props.children}
        }
    }
}
