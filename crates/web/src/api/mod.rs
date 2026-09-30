//! Server functions, one file per feature module. Each file holds its wire types and server
//! function signatures for both builds, and its server-only code in one `mod server`. Routes
//! that answer with something other than data, like `artwork`, are server build only.

pub mod add;
#[cfg(feature = "server")]
pub(crate) mod artwork;
pub mod downloads;
pub mod history;
pub mod library;
pub mod rename;
pub mod review;
pub mod settings;

use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use {
    crate::state::{AddSettings, Dep},
    yokoku_domain::Clock,
    yokoku_downloads::Downloads,
    yokoku_events::{History, QueueChanges},
    yokoku_library::{Artworks, Calendar, Library, MetadataService},
    yokoku_media::{Deleter, Importer, Prober, Renamer, Reviewer, RootFolders},
};

/// What a failed server function call tells the user: the server's message, or that it was not reached.
pub fn failure(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message.clone(),
        _ => "The server could not be reached; try again".to_owned(),
    }
}

/// A library use case's error as a message for the user; unexpected ones go to the log.
#[cfg(feature = "server")]
fn library_failure(error: yokoku_library::LibraryError, doing: &str) -> ServerFnError {
    use dioxus::logger::tracing::error;
    use yokoku_library::{LibraryError, ports::MetadataError};

    let message = match &error {
        LibraryError::AlreadyInLibrary(_) => "It is already in the library".to_owned(),
        LibraryError::FolderTaken(path) => format!("{} already belongs to another item", path.display()),
        LibraryError::InvalidFolder(_) => "Give a folder name without slashes".to_owned(),
        LibraryError::SeriesNotFound(_) | LibraryError::MovieNotFound(_) => "It is no longer in the library".to_owned(),
        LibraryError::SeasonNotFound(_) | LibraryError::EpisodeNotFound(_) => {
            "The metadata source no longer lists it; reload the page".to_owned()
        },
        LibraryError::Metadata(MetadataError::NotFound(_)) => "The metadata source no longer has it".to_owned(),
        LibraryError::Metadata(MetadataError::Unavailable(_)) => {
            "The metadata source could not be reached; try again".to_owned()
        },
        LibraryError::Metadata(MetadataError::Refused(_)) => {
            "The metadata source refused the request; check the token".to_owned()
        },
        _ => {
            error!(%error, "{doing} failed");
            "Something went wrong; the server log has the cause".to_owned()
        },
    };
    ServerFnError::new(message)
}
