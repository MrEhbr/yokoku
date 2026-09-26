use std::sync::Arc;

use tokio::sync::watch;

/// Wakes event deliveries after new events are committed.
#[derive(Debug, Clone)]
pub struct NewEvents {
    sender: Arc<watch::Sender<()>>,
}

impl NewEvents {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(());
        Self { sender: Arc::new(sender) }
    }

    pub fn notify(&self) {
        self.sender.send_replace(());
    }

    pub fn listen(&self) -> Listener {
        Listener { receiver: self.sender.subscribe(), _sender: Arc::clone(&self.sender) }
    }
}

impl Default for NewEvents {
    fn default() -> Self {
        Self::new()
    }
}

/// A notification sent after `mark_seen` is never lost.
#[derive(Debug)]
pub struct Listener {
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
