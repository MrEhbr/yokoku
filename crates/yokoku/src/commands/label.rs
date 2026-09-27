use owo_colors::{Style, Styled};
use yokoku_domain::{Confidence, FileStatus, MediaKind, MovieStatus, ReleaseKind, SeriesStatus};
use yokoku_downloads::{Download, DownloadState};
use yokoku_library::LibraryStatus;
use yokoku_media::{ImportStatus, RootKind};

const PLAIN: Style = Style::new();
const GOOD: Style = Style::new().green();
const BAD: Style = Style::new().red();
const PENDING: Style = Style::new().yellow();
const QUIET: Style = Style::new().dimmed();

/// A value as command output shows it, colored by what it means.
pub trait Label {
    fn label(&self) -> Styled<&'static str>;
}

impl Label for MediaKind {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::Series => PLAIN.style("series"),
            Self::Movie => PLAIN.style("movie"),
        }
    }
}

impl Label for RootKind {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::Series => PLAIN.style("series"),
            Self::Movies => PLAIN.style("movies"),
        }
    }
}

impl Label for LibraryStatus {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::Series(SeriesStatus::Continuing) => GOOD.style("continuing"),
            Self::Series(SeriesStatus::OnBreak) => PENDING.style("on break"),
            Self::Series(SeriesStatus::Ended) => QUIET.style("ended"),
            Self::Movie(MovieStatus::Announced) => PENDING.style("announced"),
            Self::Movie(MovieStatus::InCinemas) => PENDING.style("in cinemas"),
            Self::Movie(MovieStatus::Released) => GOOD.style("released"),
        }
    }
}

impl Label for FileStatus {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::Downloaded => GOOD.style("downloaded"),
            Self::Missing => BAD.style("missing"),
            Self::Upcoming => PENDING.style("upcoming"),
        }
    }
}

impl Label for ReleaseKind {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::Cinema => PLAIN.style("cinema release"),
            Self::Digital => PLAIN.style("digital release"),
            Self::Physical => PLAIN.style("physical release"),
        }
    }
}

impl Label for ImportStatus {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::NeedsReview => PENDING.style("review"),
            Self::Approved => PLAIN.style("approved"),
            Self::Importing => PLAIN.style("importing"),
            Self::Done => GOOD.style("done"),
            Self::Failed => BAD.style("failed"),
        }
    }
}

impl Label for Confidence {
    fn label(&self) -> Styled<&'static str> {
        match self {
            Self::Unknown => BAD.style("unknown"),
            Self::Guess => PENDING.style("guess"),
            Self::Certain => GOOD.style("certain"),
        }
    }
}

/// The download's state; `error` whenever Transmission reports one.
impl Label for Download {
    fn label(&self) -> Styled<&'static str> {
        if self.status.error.is_some() {
            return BAD.style("error");
        }
        match self.status.state {
            DownloadState::Queued => QUIET.style("queued"),
            DownloadState::Checking => PLAIN.style("checking"),
            DownloadState::Downloading => PLAIN.style("downloading"),
            DownloadState::Seeding => GOOD.style("seeding"),
            DownloadState::Stopped if self.completed_at.is_some() => GOOD.style("finished"),
            DownloadState::Stopped => QUIET.style("paused"),
            DownloadState::Removed => QUIET.style("removed"),
        }
    }
}

/// `monitored`, or a dimmed `unmonitored`.
pub fn monitored(monitored: bool) -> Styled<&'static str> {
    if monitored { GOOD.style("monitored") } else { QUIET.style("unmonitored") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_the_text_inside_its_color() {
        assert_eq!(format!("{:<10}|", FileStatus::Missing.label()), "\x1b[31mmissing   \x1b[0m|");
    }

    #[test]
    fn a_plain_label_has_no_escape_codes() {
        assert_eq!(format!("{:<8}|", MediaKind::Movie.label()), "movie   |");
    }
}
