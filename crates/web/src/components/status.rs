use dioxus::prelude::*;

/// A status category's tone. Each category (file availability, lifecycle, monitoring,
/// confidence, operation) maps its own values to tones; never merge categories into one badge.
#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Success,
    Warning,
    Danger,
    Info,
    Muted,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Self::Success => "text-success",
            Self::Warning => "text-warning",
            Self::Danger => "text-danger",
            Self::Info => "text-info",
            Self::Muted => "text-muted",
        }
    }

    /// A symbol that keeps the status readable without color.
    fn symbol(self) -> &'static str {
        match self {
            Self::Success => "✓",
            Self::Warning => "!",
            Self::Danger => "×",
            Self::Info => "○",
            Self::Muted => "—",
        }
    }
}

/// A status: symbol and label in the tone's color.
#[component]
pub fn Status(tone: Tone, label: String) -> Element {
    let (class, symbol) = (tone.class(), tone.symbol());
    rsx! {
        span { class: "inline-flex max-w-full items-center gap-1.5 text-caption font-medium {class}",
            span { aria_hidden: "true", "{symbol}" }
            span { class: "truncate", "{label}" }
        }
    }
}
