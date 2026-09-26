#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Downloaded,
    Missing,
    Upcoming,
}
