//! Event handlers, subscriptions and event delivery; re-exports the event contract from
//! `yokoku_domain::events`.

pub mod correlation;
mod delivery;
mod event_log;
mod history;
mod publisher;
mod signal;
mod spool;
mod subscriber;
mod subscription;

pub use delivery::{Delivery, DeliveryConfig};
pub use event_log::{Correlated, DeliveryFailure, EventId, EventLog, Recorded};
pub use history::History;
pub use publisher::Publisher;
pub use signal::{Listener, NewEvents};
pub use spool::EventSpool;
pub use subscriber::{HandlerError, Subscriber};
pub use subscription::{Handler, Subscription};
pub use yokoku_domain::{
    CorrelationId,
    events::{
        DeleteReason, DownloadCompleted, EpisodesRenumbered, Event, EventKind, FileDeleted, FileRenamed, FilesFound,
        FilesImported, ImportFailed, ImportNeedsReview, LinkedFile, MovieAdded, MovieRemoved, RenumberedFile,
        SeriesAdded, SeriesRemoved, SettingsChanged, TorrentAdded, TorrentRemoved,
    },
};
