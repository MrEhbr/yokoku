use jiff::Zoned;

pub trait Clock: Send + Sync {
    /// The current time in the user's time zone.
    fn now(&self) -> Zoned;
}
