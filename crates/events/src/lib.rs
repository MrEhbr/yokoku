//! Event handlers, subscriptions and event delivery; re-exports the event contract from
//! `yokoku_domain::events`.

pub mod correlation;
mod delivery;
mod history;
mod log;
mod publisher;
mod signal;
mod spool;
mod subscriber;
mod subscription;

pub use delivery::{Delivery, DeliveryConfig};
pub use history::History;
pub use log::{Correlated, DeliveryFailure, EventId, EventLog, Recorded};
pub use publisher::Publisher;
pub use signal::{Listener, NewEvents};
pub use spool::EventSpool;
pub use subscriber::{HandlerError, Subscriber};
pub use subscription::{Handler, Subscription};
pub use yokoku_domain::{
    CorrelationId,
    events::{
        DeleteReason, DownloadCompleted, Event, EventKind, FileDeleted, FileRenamed, FilesFound, FilesImported,
        ImportFailed, ImportNeedsReview, LinkedFile, MovieAdded, MovieRemoved, SeriesAdded, SeriesRemoved,
        TorrentAdded, TorrentRemoved,
    },
};
