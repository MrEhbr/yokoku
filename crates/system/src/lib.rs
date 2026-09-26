//! Filesystem, clock, media probing and media server adapters.

mod clock;
mod fs;

pub use clock::SystemClock;
pub use fs::LocalFileSystem;
