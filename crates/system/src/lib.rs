//! Filesystem, library lock, clock, media probing, event spool and artwork cache adapters.

mod artwork;
mod clock;
mod fs;
mod lock;
mod probe;
mod spool;

pub use artwork::ArtworkFiles;
pub use clock::{ClockSettings, SystemClock};
pub use fs::LocalFileSystem;
pub use lock::LockFile;
pub use probe::{FfProbe, ProbeSettings};
pub use spool::FileSpool;
