use std::sync::Arc;

use tokio::sync::watch;

/// Wakes event deliveries after new events are committed.
#[derive(Debug, Clone)]
pub(crate) struct NewEvents {
    sender: Arc<watch::Sender<()>>,
}

impl NewEvents {
    pub(crate) fn new() -> Self {
        let (sender, _) = watch::channel(());
        Self { sender: Arc::new(sender) }
    }

    pub(crate) fn notify(&self) {
        self.sender.send_replace(());
    }

    pub(crate) fn listen(&self) -> Listener {
        Listener { receiver: self.sender.subscribe(), _sender: Arc::clone(&self.sender) }
    }
}

/// A notification sent after `mark_seen` is never lost.
#[derive(Debug)]
pub(crate) struct Listener {
    receiver: watch::Receiver<()>,
    _sender: Arc<watch::Sender<()>>,
}

impl Listener {
    pub(crate) fn mark_seen(&mut self) {
        self.receiver.borrow_and_update();
    }

    pub(crate) async fn changed(&mut self) {
        self.receiver.changed().await.expect("the listener owns a sender");
    }
}

/// Wakes watchers after stored downloads or imports change, in this process.
#[derive(Debug, Clone)]
pub struct QueueChanges {
    sender: Arc<watch::Sender<()>>,
}

impl QueueChanges {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(());
        Self { sender: Arc::new(sender) }
    }

    pub fn notify(&self) {
        self.sender.send_replace(());
    }

    /// Sees each change made after this call.
    pub fn watch(&self) -> watch::Receiver<()> {
        self.sender.subscribe()
    }
}

impl Default for QueueChanges {
    fn default() -> Self {
        Self::new()
    }
}
