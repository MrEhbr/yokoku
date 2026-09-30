use std::path::{Path, PathBuf};

use proptest::prelude::*;
use rstest::rstest;
use yokoku_domain::SubtitleTags;
use yokoku_naming::subtitle_path;

fn tags(language: Option<&str>, sdh: bool, forced: bool) -> SubtitleTags {
    SubtitleTags { language: language.map(Into::into), sdh, forced }
}

#[rstest]
#[case::language(tags(Some("en"), false, false), "srt", "Dune (2021).en.srt")]
#[case::region(tags(Some("pt-BR"), false, false), "srt", "Dune (2021).pt-br.srt")]
#[case::flags(tags(Some("en"), true, true), "ASS", "Dune (2021).en.sdh.forced.ass")]
#[case::forced_only(tags(None, false, true), ".srt", "Dune (2021).forced.srt")]
#[case::no_tags(tags(None, false, false), "srt", "Dune (2021).srt")]
#[case::unsafe_language(tags(Some("e/n"), false, false), "srt", "Dune (2021).en.srt")]
fn subtitles_match_their_video(#[case] tags: SubtitleTags, #[case] extension: &str, #[case] expected: &str) {
    let video = Path::new("Dune (2021)/Dune (2021).mkv");

    assert_eq!(subtitle_path(video, &tags, extension), PathBuf::from("Dune (2021)").join(expected));
}

proptest! {
    #[test]
    fn a_subtitle_sits_beside_its_video_and_starts_with_its_name(
        stem in "[A-Za-z0-9 ().-]{1,30}",
        language in proptest::option::of(".*"),
        sdh in any::<bool>(),
        forced in any::<bool>(),
        extension in ".*",
    ) {
        let video = Path::new("/tv/Frieren (2023)").join(format!("{stem}.mkv"));

        let subtitle = subtitle_path(&video, &tags(language.as_deref(), sdh, forced), &extension);

        prop_assert_eq!(subtitle.parent(), video.parent());
        let name = subtitle.file_name().unwrap().to_string_lossy().into_owned();
        prop_assert!(name.starts_with(&stem), "{name}");
    }
}
