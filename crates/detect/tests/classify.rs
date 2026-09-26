use std::path::PathBuf;

use proptest::prelude::*;
use rstest::rstest;
use yokoku_detect::{Classified, DownloadFile};
use yokoku_domain::SubtitleTags;

fn files(paths: &[&str]) -> Vec<DownloadFile> {
    paths.iter().map(|path| DownloadFile { path: PathBuf::from(path), size: 1 }).collect()
}

fn videos(classified: &Classified) -> Vec<&str> {
    classified.videos.iter().map(|video| video.path.to_str().unwrap()).collect()
}

fn ignored(classified: &Classified) -> Vec<&str> {
    let mut ignored: Vec<_> = classified.ignored.iter().map(|path| path.to_str().unwrap()).collect();
    ignored.sort_unstable();
    ignored
}

fn subtitles_of<'a>(classified: &'a Classified, video: &str) -> Vec<&'a str> {
    let video = classified.videos.iter().find(|candidate| candidate.path.to_str() == Some(video)).unwrap();
    video.subtitles.iter().map(|subtitle| subtitle.path.to_str().unwrap()).collect()
}

#[test]
fn keeps_videos_and_ignores_junk() {
    let classified = Classified::from_files(&files(&[
        "Show.S01/Show.S01E02.mkv",
        "Show.S01/Show.S01E01.MP4",
        "Show.S01/Show.S01E01.sample.mkv",
        "Show.S01/Sample/show-s01e01.mkv",
        "Show.S01/Featurettes/Making Of.mkv",
        "Show.S01/Show.S01.nfo",
        "Show.S01/poster.jpg",
        "Show.S01/RARBG.txt",
        "Show.S01/Visit us.url",
    ]));

    assert_eq!(videos(&classified), ["Show.S01/Show.S01E01.MP4", "Show.S01/Show.S01E02.mkv"]);
    assert_eq!(
        ignored(&classified),
        [
            "Show.S01/Featurettes/Making Of.mkv",
            "Show.S01/RARBG.txt",
            "Show.S01/Sample/show-s01e01.mkv",
            "Show.S01/Show.S01.nfo",
            "Show.S01/Show.S01E01.sample.mkv",
            "Show.S01/Visit us.url",
            "Show.S01/poster.jpg",
        ]
    );
}

#[test]
fn a_title_containing_sample_is_not_a_sample() {
    let classified = Classified::from_files(&files(&["Free.Samples.2012.1080p.mkv"]));

    assert_eq!(videos(&classified), ["Free.Samples.2012.1080p.mkv"]);
}

#[test]
fn attaches_subtitles_named_after_their_video() {
    let classified = Classified::from_files(&files(&[
        "Show.S01E01.mkv",
        "Show.S01E02.mkv",
        "Show.S01E01.en.srt",
        "Show.S01E01.ru.forced.srt",
        "Show.S01E02.en.srt",
    ]));

    assert_eq!(subtitles_of(&classified, "Show.S01E01.mkv"), ["Show.S01E01.en.srt", "Show.S01E01.ru.forced.srt"]);
    assert_eq!(subtitles_of(&classified, "Show.S01E02.mkv"), ["Show.S01E02.en.srt"]);
    assert!(classified.ignored.is_empty());
}

#[test]
fn attaches_subtitles_in_folders_named_after_their_video() {
    let classified = Classified::from_files(&files(&[
        "Show.S01/Show.S01E01.mkv",
        "Show.S01/Show.S01E02.mkv",
        "Show.S01/Subs/Show.S01E02/2_English.srt",
        "Show.S01/Subs/Show.S01E02/3_Russian.srt",
    ]));

    assert_eq!(
        subtitles_of(&classified, "Show.S01/Show.S01E02.mkv"),
        ["Show.S01/Subs/Show.S01E02/2_English.srt", "Show.S01/Subs/Show.S01E02/3_Russian.srt"]
    );
    assert!(subtitles_of(&classified, "Show.S01/Show.S01E01.mkv").is_empty());
}

#[test]
fn attaches_any_subtitle_to_a_single_video() {
    let classified =
        Classified::from_files(&files(&["Movie.2021/Movie.2021.1080p.mkv", "Movie.2021/Subs/English.srt"]));

    assert_eq!(subtitles_of(&classified, "Movie.2021/Movie.2021.1080p.mkv"), ["Movie.2021/Subs/English.srt"]);
}

#[test]
fn ignores_subtitles_that_match_no_video() {
    let classified = Classified::from_files(&files(&["Show.S01E01.mkv", "Show.S01E02.mkv", "Subs/English.srt"]));

    assert_eq!(ignored(&classified), ["Subs/English.srt"]);
}

#[test]
fn ignores_subtitles_inside_samples() {
    let classified = Classified::from_files(&files(&["Movie.mkv", "Sample/Movie.en.srt"]));

    assert_eq!(ignored(&classified), ["Sample/Movie.en.srt"]);
}

#[rstest]
#[case::code("Movie.en.srt", Some("en"), false, false)]
#[case::three_letter_code("Movie.eng.forced.srt", Some("en"), false, true)]
#[case::english_name("Movie.English.SDH.srt", Some("en"), true, false)]
#[case::numbered_name("2_Russian.srt", Some("ru"), false, false)]
#[case::region("Movie.pt-BR.srt", Some("pt-br"), false, false)]
#[case::bibliographic_code("Movie.gre.srt", Some("el"), false, false)]
#[case::any_iso_639_3_code("Movie.vie.srt", Some("vi"), false, false)]
#[case::any_english_name("Movie.Hindi.srt", Some("hi"), false, false)]
#[case::hearing_impaired("Movie.en.hi.srt", Some("en"), true, false)]
#[case::closed_captions("Movie.cc.srt", None, true, false)]
#[case::no_tags("Movie.srt", None, false, false)]
#[case::unknown_word("Movie.xyz.srt", None, false, false)]
#[case::title_word_is_not_a_language("Show.S01E01.GO.srt", None, false, false)]
fn reads_subtitle_language_and_flags(
    #[case] subtitle: &str,
    #[case] language: Option<&str>,
    #[case] sdh: bool,
    #[case] forced: bool,
) {
    let classified = Classified::from_files(&files(&["Movie.mkv", subtitle]));

    let tags = &classified.videos[0].subtitles[0].tags;
    assert_eq!(tags, &SubtitleTags { language: language.map(Into::into), sdh, forced });
}

fn any_path() -> impl Strategy<Value = String> {
    let segment = "[A-Za-z0-9 ._-]{1,12}";
    let extension = prop::sample::select(vec!["mkv", "mp4", "srt", "ass", "nfo", "jpg", "txt", ""]);
    (prop::collection::vec(segment, 0..3), segment, extension).prop_map(|(folders, name, extension)| {
        let mut path = folders.join("/");
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(&name);
        if !extension.is_empty() {
            path.push('.');
            path.push_str(extension);
        }
        path
    })
}

proptest! {
    #[test]
    fn every_file_lands_in_exactly_one_place(paths in prop::collection::btree_set(any_path(), 0..12)) {
        let input: Vec<_> = paths.iter().map(|path| DownloadFile { path: PathBuf::from(path), size: 1 }).collect();

        let classified = Classified::from_files(&input);

        let mut output: Vec<PathBuf> = classified.ignored.clone();
        for video in &classified.videos {
            output.push(video.path.clone());
            output.extend(video.subtitles.iter().map(|subtitle| subtitle.path.clone()));
        }
        output.sort();
        let mut expected: Vec<PathBuf> = input.into_iter().map(|file| file.path).collect();
        expected.sort();
        prop_assert_eq!(output, expected);
    }
}
