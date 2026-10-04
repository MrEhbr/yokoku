use std::path::{Path, PathBuf};

use proptest::prelude::*;
use rstest::rstest;
use yokoku_domain::naming::sidecar_path;

#[rstest]
#[case::suffix("RUS.Sound.AniLibria", "mka", "Dune (2021).RUS.Sound.AniLibria.mka")]
#[case::flags("en.forced", "srt", "Dune (2021).en.forced.srt")]
#[case::no_suffix("", "srt", "Dune (2021).srt")]
#[case::extension_lowercased("en", "ASS", "Dune (2021).en.ass")]
#[case::dotted_extension("en", ".srt", "Dune (2021).en.srt")]
#[case::unsafe_part("e/n", "srt", "Dune (2021).e-n.srt")]
fn sidecars_match_their_video(#[case] suffix: &str, #[case] extension: &str, #[case] expected: &str) {
    let video = Path::new("Dune (2021)/Dune (2021).mkv");

    assert_eq!(sidecar_path(video, suffix, extension), PathBuf::from("Dune (2021)").join(expected));
}

proptest! {
    #[test]
    fn a_sidecar_sits_beside_its_video_and_starts_with_its_name(
        stem in "[A-Za-z0-9][A-Za-z0-9 ().-]{0,29}",
        suffix in ".*",
        extension in ".*",
    ) {
        let video = Path::new("/tv/Frieren (2023)").join(format!("{stem}.mkv"));

        let sidecar = sidecar_path(&video, &suffix, &extension);

        prop_assert_eq!(sidecar.parent(), video.parent());
        let name = sidecar.file_name().unwrap().to_string_lossy().into_owned();
        prop_assert!(name.starts_with(&stem), "{name}");
        prop_assert!(name.len() <= 255, "{} bytes", name.len());
    }
}
