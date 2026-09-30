#![allow(dead_code)]

use jiff::Timestamp;
use rstest::fixture;
use yokoku_infra::db::Database;

/// A file size that does not fit in 32 bits.
pub const SIZE_BEYOND_U32: u64 = 1 << 33;

#[fixture]
pub async fn db() -> Database {
    Database::open_in_memory().await.unwrap()
}

/// A time with nanoseconds, which storage must keep.
pub fn now() -> Timestamp {
    "2026-09-26T12:00:00.123456789Z".parse().unwrap()
}

/// Runs `future` to completion, for a property test.
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(future)
}
