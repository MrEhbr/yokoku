use yokoku_domain::StorageError;

use crate::LibraryError;

const ATTEMPTS: u32 = 5;

/// Runs `attempt` again while its save loses a race with another save, up to five times in all;
/// each attempt must load what it changes.
pub(crate) async fn on_conflict<T, F>(mut attempt: impl FnMut() -> F) -> Result<T, LibraryError>
where
    F: Future<Output = Result<T, LibraryError>>,
{
    let mut left = ATTEMPTS;
    loop {
        match attempt().await {
            Err(LibraryError::Storage(StorageError::Conflict)) if left > 1 => left -= 1,
            result => return result,
        }
    }
}
