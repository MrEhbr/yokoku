//! Server functions, one file per feature module. Each file holds its wire types and server
//! function signatures for both builds, and its server-only code in one `mod server`. Routes
//! that answer with something other than data, like `artwork`, are server build only.

#[cfg(feature = "server")]
pub(crate) mod artwork;
pub mod library;

#[cfg(feature = "server")]
use {
    crate::state::Dep,
    yokoku_library::{Artworks, Calendar, Library},
    yokoku_media::Prober,
};
