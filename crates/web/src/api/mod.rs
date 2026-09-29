//! Server functions, one file per feature module. Each file holds its wire types and server
//! function signatures for both builds, and its server-only code in one `mod server`. Routes
//! that answer with something other than data, like `artwork`, are server build only.

pub mod add;
#[cfg(feature = "server")]
pub(crate) mod artwork;
pub mod downloads;
pub mod history;
pub mod library;

use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use {
    crate::state::{AddSettings, Dep},
    yokoku_domain::Clock,
    yokoku_downloads::Downloads,
    yokoku_events::{History, QueueChanges},
    yokoku_library::{Artworks, Calendar, Library, MetadataService},
    yokoku_media::{Importer, Prober, Reviewer, RootFolders},
};

/// What a failed server function call tells the user: the server's message, or that it was not reached.
pub fn failure(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message.clone(),
        _ => "The server could not be reached; try again".to_owned(),
    }
}
