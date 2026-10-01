use jiff::Timestamp;
use rstest::rstest;
use support::db;
use uuid::Uuid;
use yokoku_core::{
    integrations::ports::{Watched, WatchedStore},
    media::{
        MediaFile,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{FileTarget, MediaFileId, MovieId};
use yokoku_infra::db::Database;

use crate::support;

fn file(n: u128) -> MediaFile {
    MediaFile {
        id: MediaFileId(Uuid::from_u128(n)),
        path: format!("/movies/{n}.mkv").into(),
        size: 1,
        target: FileTarget::Movie(MovieId(Uuid::from_u128(n))),
        added_at: Timestamp::UNIX_EPOCH,
    }
}

fn watched(file: &MediaFile, second: Option<i64>) -> Watched {
    Watched { file: file.id, at: second.map(|second| Timestamp::from_second(second).unwrap()) }
}

async fn stored(db: &Database, files: &[MediaFile]) {
    db.save(&Changes { added_files: files.to_vec(), ..Changes::default() }).await.unwrap();
}

#[rstest]
#[tokio::test]
async fn replacing_keeps_only_the_new_files_that_are_stored(#[future(awt)] db: Database) {
    let (a, b, c, gone) = (file(1), file(2), file(3), file(4));
    stored(&db, &[a.clone(), b.clone(), c.clone()]).await;
    db.replace_watched(&[watched(&a, Some(10)), watched(&b, Some(20))]).await.unwrap();

    db.replace_watched(&[watched(&b, Some(30)), watched(&c, None), watched(&gone, Some(40))]).await.unwrap();

    assert_eq!(db.watched().await.unwrap(), [watched(&b, Some(30)), watched(&c, None)]);
}

#[rstest]
#[tokio::test]
async fn a_watched_file_goes_with_its_file(#[future(awt)] db: Database) {
    let (a, b) = (file(1), file(2));
    stored(&db, &[a.clone(), b.clone()]).await;
    db.replace_watched(&[watched(&a, Some(10)), watched(&b, Some(20))]).await.unwrap();

    db.save(&Changes { removed_files: vec![a.id], ..Changes::default() }).await.unwrap();

    assert_eq!(db.watched().await.unwrap(), [watched(&b, Some(20))]);
}
