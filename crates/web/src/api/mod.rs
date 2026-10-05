//! Server functions, one file per feature module. Each file holds its wire types and server
//! function signatures for both builds, and its server-only code in one `mod server`. Routes
//! that answer with something other than data, like `artwork`, are server build only.

pub mod add;
#[cfg(feature = "server")]
pub(crate) mod artwork;
pub mod downloads;
pub mod history;
pub mod library;
pub mod releases;
pub mod rename;
pub mod review;
pub mod settings;

use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use {
    crate::state::{AddSettings, Dep},
    tokio_util::sync::CancellationToken,
    yokoku_core::downloads::{Downloads, ReleaseSearch},
    yokoku_core::events::{History, QueueChanges},
    yokoku_core::library::{Artworks, Calendar, Library, MetadataService},
    yokoku_core::media::{Deleter, Importer, Prober, Renamer, Reviewer, RootFolders, Scanner},
    yokoku_domain::Clock,
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
fn library_failure(error: yokoku_core::library::LibraryError, doing: &str) -> ServerFnError {
    use yokoku_core::library::{LibraryError, ports::MetadataError};

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
        _ => return unexpected(&error, doing),
    };
    ServerFnError::new(message)
}

/// An unexpected error as a message for the user; the error and what was being done go to the log.
#[cfg(feature = "server")]
fn unexpected(error: &dyn std::fmt::Display, doing: &str) -> ServerFnError {
    dioxus::logger::tracing::error!(%error, "{doing} failed");
    ServerFnError::new("Something went wrong; the server log has the cause")
}

#[cfg(feature = "server")]
fn root_listing_failed(error: yokoku_core::media::MediaError) -> ServerFnError {
    dioxus::logger::tracing::error!(%error, "listing root folders failed");
    ServerFnError::new("The root folders could not be loaded")
}
