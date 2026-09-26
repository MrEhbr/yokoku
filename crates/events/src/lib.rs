//! Subscriber trait and event delivery; re-exports the event contract from `yokoku_domain::events`.

mod delivery;
mod history;
mod log;
mod signal;
mod subscriber;

pub use delivery::{Delivery, DeliveryConfig};
pub use history::History;
pub use log::{EventId, EventLog, Failure, Recorded};
pub use signal::{Listener, NewEvents};
pub use subscriber::{HandlerError, Subscriber};
pub use yokoku_domain::events::{
    DeleteReason, DownloadCompleted, Event, EventKind, FileDeleted, FileRenamed, FilesFound, FilesImported,
    ImportFailed, ImportNeedsReview, LinkedFile, MovieAdded, MovieRemoved, SeriesAdded, SeriesRemoved, TorrentAdded,
    TorrentRemoved,
};
