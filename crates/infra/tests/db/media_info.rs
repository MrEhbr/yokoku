use std::time::Duration;

use jiff::Timestamp;
use proptest::prelude::*;
use rstest::rstest;
use support::{SIZE_BEYOND_U32, block_on, db};
use uuid::Uuid;
use yokoku_domain::{FileTarget, MediaFileId, MovieId};
use yokoku_infra::db::Database;
use yokoku_media::{
    AudioStream, MediaFile, MediaInfo, SubtitleStream, VideoStream,
    ports::{Changes, MediaRepo},
};

use crate::support;

fn file(path: &str) -> MediaFile {
    MediaFile {
        id: MediaFileId::generate(),
        path: path.into(),
        size: SIZE_BEYOND_U32,
        target: FileTarget::Movie(MovieId(Uuid::from_u128(1))),
        added_at: Timestamp::UNIX_EPOCH,
    }
}

fn info() -> MediaInfo {
    MediaInfo {
        duration: Some(Duration::from_millis(9_345_678)),
        video: Some(VideoStream { codec: "hevc".into(), width: 3840, height: 2160 }),
        audio: vec![
            AudioStream { codec: "eac3".into(), language: Some("eng".into()), channels: 6 },
            AudioStream { codec: "aac".into(), language: None, channels: 2 },
        ],
        subtitles: vec![SubtitleStream { codec: "subrip".into(), language: Some("rus".into()), forced: true }],
    }
}

async fn stored(db: &Database, files: &[MediaFile]) {
    db.save(&Changes { added_files: files.to_vec(), ..Changes::default() }).await.unwrap();
}

#[rstest]
#[tokio::test]
async fn details_are_replaced_and_go_with_their_file(#[future(awt)] db: Database) {
    let dune = file("/movies/Dune.mkv");
    stored(&db, std::slice::from_ref(&dune)).await;

    db.save_media_info(dune.id, &info()).await.unwrap();
    db.save_media_info(dune.id, &MediaInfo::default()).await.unwrap();
    let replaced = db.media_info(dune.id).await.unwrap();
    db.save(&Changes { removed_files: vec![dune.id], ..Changes::default() }).await.unwrap();

    assert_eq!(replaced, Some(MediaInfo::default()));
    assert_eq!(db.media_info(dune.id).await.unwrap(), None);
}

#[rstest]
#[tokio::test]
async fn details_of_a_file_no_longer_stored_are_left_out(#[future(awt)] db: Database) {
    let gone = MediaFileId::generate();

    db.save_media_info(gone, &info()).await.unwrap();

    assert_eq!(db.media_info(gone).await.unwrap(), None);
}

#[rstest]
#[tokio::test]
async fn files_never_probed_are_listed_by_path(#[future(awt)] db: Database) {
    let (a, b, c) = (file("/movies/a.mkv"), file("/movies/b.mkv"), file("/movies/c.mkv"));
    stored(&db, &[c.clone(), b.clone(), a.clone()]).await;
    db.save_media_info(b.id, &info()).await.unwrap();

    assert_eq!(db.files_without_media_info().await.unwrap(), [a, c]);
}

fn any_language() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[a-z]{3}")
}

fn any_info() -> impl Strategy<Value = MediaInfo> {
    let video = prop::option::of(("[a-z0-9]{1,8}", any::<u32>(), any::<u32>()))
        .prop_map(|video| video.map(|(codec, width, height)| VideoStream { codec, width, height }));
    let audio = prop::collection::vec(("[a-z0-9]{1,8}", any_language(), any::<u16>()), 0..4).prop_map(|streams| {
        streams.into_iter().map(|(codec, language, channels)| AudioStream { codec, language, channels }).collect()
    });
    let subtitles = prop::collection::vec(("[a-z_]{1,12}", any_language(), any::<bool>()), 0..4).prop_map(|streams| {
        streams.into_iter().map(|(codec, language, forced)| SubtitleStream { codec, language, forced }).collect()
    });
    (prop::option::of(0..i64::MAX as u64 / 1000), video, audio, subtitles).prop_map(
        |(duration, video, audio, subtitles)| MediaInfo {
            duration: duration.map(Duration::from_millis),
            video,
            audio,
            subtitles,
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn stored_details_read_back_unchanged(info in any_info()) {
        let stored_info = block_on(async {
            let db = Database::open_in_memory().await.unwrap();
            let dune = file("/movies/Dune.mkv");
            stored(&db, std::slice::from_ref(&dune)).await;
            db.save_media_info(dune.id, &info).await.unwrap();
            db.media_info(dune.id).await.unwrap()
        });

        prop_assert_eq!(stored_info, Some(info));
    }
}
