#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Downloaded,
    Missing,
    Upcoming,
}

crate::string_enum!(FileStatus, "file status" {
    Downloaded => "downloaded",
    Missing => "missing",
    Upcoming => "upcoming",
});
