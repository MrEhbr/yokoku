use std::error::Error;

use async_trait::async_trait;
use yokoku_domain::events::Event;

pub type HandlerError = Box<dyn Error + Send + Sync>;

/// Receives every event in log order, at least once, so handling must be idempotent; a failure
/// fails the delivery. An event the delivery gave up on is tried again later, after newer events.
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Keys the stored delivery position; a new name delivers the whole log again.
    fn name(&self) -> &'static str;

    async fn handle(&self, event: &Event) -> Result<(), HandlerError>;
}
