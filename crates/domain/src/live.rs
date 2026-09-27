use std::{fmt, sync::Arc};

/// A setting read each time it is used, so a change made while the app runs takes effect.
pub struct Live<T>(Arc<dyn Fn() -> T + Send + Sync>);

impl<T> Live<T> {
    pub fn new(read: impl Fn() -> T + Send + Sync + 'static) -> Self {
        Self(Arc::new(read))
    }

    pub fn current(&self) -> T {
        (self.0)()
    }
}

impl<T: Clone + Send + Sync + 'static> Live<T> {
    /// Always `value`.
    pub fn fixed(value: T) -> Self {
        Self::new(move || value.clone())
    }
}

impl<T> Clone for Live<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> fmt::Debug for Live<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Live")
    }
}
