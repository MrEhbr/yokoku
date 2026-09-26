use std::error::Error;

use async_trait::async_trait;

use crate::Recorded;

pub type HandlerError = Box<dyn Error + Send + Sync>;

/// Receives every event in log order, at least once; handlers must be idempotent.
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Stable identity; the delivery position is stored under this name.
    fn name(&self) -> &'static str;

    async fn handle(&self, event: &Recorded) -> Result<(), HandlerError>;
}
