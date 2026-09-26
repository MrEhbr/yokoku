use rstest::rstest;
use uuid::Uuid;
use yokoku_domain::{
    EpisodeSpan, FileTarget, ImportId, ItemId, MediaFileId, MovieId, SeriesId,
    events::{
        DeleteReason, Event, FileDeleted, FileRenamed, FilesFound, ImportNeedsReview, LinkedFile, MovieRemoved,
        SeriesAdded, TorrentAdded, TorrentRemoved,
    },
};

fn series() -> SeriesId {
    SeriesId(Uuid::from_u128(1))
}

fn movie() -> MovieId {
    MovieId(Uuid::from_u128(2))
}

fn episodes(first: u16) -> FileTarget {
    FileTarget::Episodes { series: series(), span: EpisodeSpan::new(1, first, first).unwrap() }
}

fn linked(target: FileTarget) -> LinkedFile {
    LinkedFile { file: MediaFileId::generate(), path: "/tv/a.mkv".into(), target }
}

#[rstest]
#[case::added(SeriesAdded { series: series(), title: "Frieren".into() }.into(), vec![ItemId::Series(series())])]
#[case::removed(MovieRemoved { movie: movie(), title: "Dune".into(), delete_files: true }.into(), vec![ItemId::Movie(movie())])]
#[case::files_of_one_series_once(
    FilesFound { files: vec![linked(episodes(1)), linked(episodes(2)), linked(FileTarget::Movie(movie()))] }.into(),
    vec![ItemId::Series(series()), ItemId::Movie(movie())],
)]
#[case::deleted(
    FileDeleted { file: MediaFileId::generate(), path: "/a".into(), target: episodes(1), reason: DeleteReason::User }.into(),
    vec![ItemId::Series(series())],
)]
#[case::renamed_before_targets(FileRenamed { file: MediaFileId::generate(), from: "/a".into(), to: "/b".into(), target: None }.into(), vec![])]
#[case::unlinked_torrent(TorrentAdded { download: yokoku_domain::DownloadId::generate(), name: "x".into(), item: None }.into(), vec![])]
#[case::removed_torrent(TorrentRemoved { download: yokoku_domain::DownloadId::generate(), name: "x".into(), item: Some(ItemId::Movie(movie())) }.into(), vec![ItemId::Movie(movie())])]
#[case::review(ImportNeedsReview { import: ImportId::generate(), source: "/x".into() }.into(), vec![])]
fn events_name_the_items_they_concern(#[case] event: Event, #[case] items: Vec<ItemId>) {
    assert_eq!(event.items(), items);
}
