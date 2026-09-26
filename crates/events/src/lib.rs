//! Event contract, subscriber trait and event delivery.

mod delivery;
mod event;
mod history;
mod log;
mod signal;
mod subscriber;

pub use delivery::{Delivery, DeliveryConfig};
pub use event::{DeleteReason, Event, LinkedFile};
pub use history::History;
pub use log::{EventId, EventLog, Failure, Recorded};
pub use signal::{Listener, NewEvents};
pub use subscriber::{HandlerError, Subscriber};
