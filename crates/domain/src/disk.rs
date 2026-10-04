use serde::{Deserialize, Serialize};

/// Bytes free and in all on a file system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskSpace {
    pub free: u64,
    /// Unknown when the download client does not report it.
    pub total: Option<u64>,
}
