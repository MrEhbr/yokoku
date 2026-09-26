/// How sure detection is about a file's match (FR-4.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Unknown,
    Guess,
    /// Imported without review.
    Certain,
}
