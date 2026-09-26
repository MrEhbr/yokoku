//! Event handlers, subscriptions and event delivery; re-exports the event contract from
//! `yokoku_domain::events`.

mod delivery;
mod history;
mod log;
mod publisher;
mod signal;
mod subscriber;
mod subscription;

pub use delivery::{Delivery, DeliveryConfig};
pub use history::History;
pub use log::{EventId, EventLog, Failure, Recorded};
pub use publisher::Publisher;
pub use signal::{Listener, NewEvents};
pub use subscriber::{HandlerError, Subscriber};
pub use subscription::{Handler, Subscription};
pub use yokoku_domain::events::{
    DeleteReason, DownloadCompleted, Event, EventKind, FileDeleted, FileRenamed, FilesFound, FilesImported,
    ImportFailed, ImportNeedsReview, LinkedFile, MovieAdded, MovieRemoved, SeriesAdded, SeriesRemoved, TorrentAdded,
    TorrentRemoved,
};
