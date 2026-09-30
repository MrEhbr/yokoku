//! Event handlers, subscriptions and event delivery; re-exports the event contract from
//! `yokoku_domain::events`.

pub mod correlation;
mod delivery;
mod event_log;
mod history;
mod publisher;
mod signal;
mod subscription;

pub use delivery::{Delivery, DeliveryConfig};
pub use event_log::{Correlated, DeliveryFailure, EventId, EventLog, EventStore, Recorded};
pub use history::History;
pub use publisher::Publisher;
pub use signal::QueueChanges;
pub use subscription::{Handler, HandlerError, Subscription};
