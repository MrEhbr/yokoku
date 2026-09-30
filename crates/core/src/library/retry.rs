use tracing::debug;
use yokoku_domain::StorageError;

use crate::library::LibraryError;

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
            Err(LibraryError::Storage(StorageError::Conflict)) if left > 1 => {
                left -= 1;
                debug!(attempts_left = left, "a concurrent save won; trying again");
            },
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use yokoku_domain::StorageError;

    use super::on_conflict;
    use crate::library::LibraryError;

    fn conflict() -> LibraryError {
        LibraryError::Storage(StorageError::Conflict)
    }

    #[tokio::test]
    async fn conflicts_are_tried_again_until_one_succeeds() {
        let attempts = Cell::new(0);

        let result = on_conflict(|| {
            attempts.set(attempts.get() + 1);
            let attempt = attempts.get();
            async move { if attempt < 3 { Err(conflict()) } else { Ok(attempt) } }
        })
        .await;

        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn five_conflicts_give_up() {
        let attempts = Cell::new(0);

        let result: Result<(), _> = on_conflict(|| {
            attempts.set(attempts.get() + 1);
            async { Err(conflict()) }
        })
        .await;

        assert!(matches!(result, Err(LibraryError::Storage(StorageError::Conflict))), "{result:?}");
        assert_eq!(attempts.get(), 5);
    }
}
