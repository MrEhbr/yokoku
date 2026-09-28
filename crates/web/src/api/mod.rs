//! Server functions, one file per feature module. Each file holds its wire types and server
//! function signatures for both builds, and its server-only code in one `mod server`.

pub mod library;

#[cfg(feature = "server")]
use {crate::state::Dep, yokoku_library::Library};
