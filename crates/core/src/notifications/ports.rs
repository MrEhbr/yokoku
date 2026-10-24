use std::error::Error;

use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    pub message: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Priority {
    #[default]
    Normal,
    /// Asks for attention, such as an import that failed.
    High,
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("notification service unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
    #[error("notification service refused the notification: {0}")]
    Refused(String),
    #[error("no notification service configured")]
    NotConfigured,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError>;
}
