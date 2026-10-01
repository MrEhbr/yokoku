//! An entry's fields as both views render them.

use dioxus::prelude::*;

use crate::{
    api::library::{FileCount, Kind},
    components::status::{self, Tone},
};

#[component]
pub(super) fn Files(files: FileCount, kind: Kind) -> Element {
    let (tone, label) = files_status(files, kind);
    rsx! {
        status::Status { tone, label }
    }
}

/// A series counts its episodes, `downloaded/wanted`; a movie says whether its file is there.
fn files_status(files: FileCount, kind: Kind) -> (Tone, String) {
    let FileCount { downloaded, missing } = files;
    let tone = match (downloaded, missing) {
        (_, 1..) => Tone::Warning,
        (0, 0) => Tone::Muted,
        _ => Tone::Success,
    };
    let label = match (kind, downloaded, missing) {
        (Kind::Movie, 0, 0) => "No file".to_owned(),
        (Kind::Movie, 0, _) => "Missing".to_owned(),
        (Kind::Movie, ..) => "Downloaded".to_owned(),
        (Kind::Series, 0, 0) => "No files".to_owned(),
        (Kind::Series, ..) => {
            let wanted = downloaded + missing;
            format!("{downloaded}/{wanted} {}", if wanted == 1 { "episode" } else { "episodes" })
        },
    };
    (tone, label)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::files_status;
    use crate::{
        api::library::{FileCount, Kind},
        components::status::Tone,
    };

    #[rstest]
    #[case::movie_downloaded(Kind::Movie, 1, 0, Tone::Success, "Downloaded")]
    #[case::movie_missing(Kind::Movie, 0, 1, Tone::Warning, "Missing")]
    #[case::movie_not_wanted_yet(Kind::Movie, 0, 0, Tone::Muted, "No file")]
    #[case::series_complete(Kind::Series, 24, 0, Tone::Success, "24/24 episodes")]
    #[case::series_partial(Kind::Series, 12, 12, Tone::Warning, "12/24 episodes")]
    #[case::series_none_downloaded(Kind::Series, 0, 3, Tone::Warning, "0/3 episodes")]
    #[case::series_one(Kind::Series, 1, 0, Tone::Success, "1/1 episode")]
    #[case::series_nothing_wanted(Kind::Series, 0, 0, Tone::Muted, "No files")]
    fn files_show_as_a_count_or_whether_the_file_is_there(
        #[case] kind: Kind,
        #[case] downloaded: usize,
        #[case] missing: usize,
        #[case] tone: Tone,
        #[case] label: &str,
    ) {
        let (shown_tone, shown_label) = files_status(FileCount { downloaded, missing }, kind);

        assert!(shown_tone == tone, "tone for {label}");
        assert_eq!(shown_label, label);
    }
}
