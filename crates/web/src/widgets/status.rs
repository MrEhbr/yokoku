use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The color and symbol of a [`status`]. Pick it from the status taxonomy in
/// DESIGN-SYSTEM.md; keep file availability, lifecycle, monitoring, confidence, and
/// operation state as separate statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Tone {
    /// A present file, a certain match, a completed operation.
    Success,
    /// Missing, a guess, or awaiting review.
    Warning,
    /// A failure or conflict.
    Danger,
    /// Upcoming, running, or informational.
    Info,
    /// Unmonitored, ended, or otherwise inactive.
    Muted,
}

impl Tone {
    fn classes(self) -> StaticClass {
        match self {
            Self::Success => class!("text-success"),
            Self::Warning => class!("text-warning"),
            Self::Danger => class!("text-danger"),
            Self::Info => class!("text-info"),
            Self::Muted => class!("text-muted"),
        }
    }

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

/// A status as symbol, label, and color, readable without the color.
///
/// ```ignore
/// view! { status(tone: Tone::Warning, label: "Missing") }
/// ```
#[component]
pub async fn status(tone: Tone, label: &str, #[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                "inline-flex items-center gap-1.5 text-caption font-medium",
                tone.classes(),
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span aria-hidden="true">(tone.symbol())</span>
            (label)
        </span>
    })
}
