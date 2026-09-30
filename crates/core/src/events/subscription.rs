use std::{error::Error, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use yokoku_domain::events::{Event, EventKind};

pub type HandlerError = Box<dyn Error + Send + Sync>;

/// Reacts to one type of event, at least once per event; handlers must be idempotent.
#[async_trait]
pub trait Handler<E: EventKind>: Send + Sync {
    async fn handle(&self, event: &E) -> Result<(), HandlerError>;
}

/// A subscriber built from handlers: it receives every event in log order, at least once, and each
/// event goes to the handlers of its type, in the order they were added; the first failure fails the
/// delivery. An event the delivery gave up on is tried again later, after newer events.
pub struct Subscription {
    name: &'static str,
    handlers: Vec<Box<dyn Dispatch>>,
}

impl Subscription {
    /// `name` keys the stored delivery position.
    pub fn new(name: &'static str) -> Self {
        Self { name, handlers: Vec::new() }
    }

    pub fn on<E: EventKind>(mut self, handler: Arc<impl Handler<E> + 'static>) -> Self {
        self.handlers.push(Box::new(On { handler, event: PhantomData }));
        self
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        for handler in &self.handlers {
            handler.dispatch(event).await?;
        }
        Ok(())
    }
}

#[async_trait]
trait Dispatch: Send + Sync {
    async fn dispatch(&self, event: &Event) -> Result<(), HandlerError>;
}

struct On<E, H> {
    handler: Arc<H>,
    event: PhantomData<fn() -> E>,
}

#[async_trait]
impl<E: EventKind, H: Handler<E> + 'static> Dispatch for On<E, H> {
    async fn dispatch(&self, event: &Event) -> Result<(), HandlerError> {
        match event.get::<E>() {
            Some(event) => self.handler.handle(event).await,
            None => Ok(()),
        }
    }
}
