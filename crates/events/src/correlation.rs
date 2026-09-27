use yokoku_domain::CorrelationId;

tokio::task_local! {
    static CURRENT: CorrelationId;
}

/// The correlation id of the command, job or event delivery this task runs for.
pub fn current() -> Option<CorrelationId> {
    CURRENT.try_with(|id| *id).ok()
}

/// Runs `work` with `id` as the current correlation id; tasks it spawns do not inherit it.
pub async fn correlate<F: Future>(id: CorrelationId, work: F) -> F::Output {
    CURRENT.scope(id, work).await
}
