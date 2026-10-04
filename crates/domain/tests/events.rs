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

#[rstest]
#[case(SeriesAdded { series: series(), title: "Frieren".into() }.into())]
#[case(ImportNeedsReview { import: ImportId::generate(), source: "/x".into() }.into())]
fn an_event_name_is_its_stored_type(#[case] event: Event) {
    let stored = serde_json::to_value(&event).unwrap();

    assert_eq!(stored["type"], event.name());
}

mod stored {
    use proptest::prelude::*;
    use rstest::rstest;
    use serde_json::json;
    use uuid::Uuid;
    use yokoku_domain::{
        DownloadId, EpisodeSpan, FileTarget, ImportId, ItemId, MediaFileId, MovieId, SeriesId, events::*,
    };

    fn linked(path: &str) -> LinkedFile {
        LinkedFile {
            file: MediaFileId(Uuid::from_u128(5)),
            path: path.into(),
            target: FileTarget::Movie(MovieId(Uuid::from_u128(3))),
        }
    }

    #[rstest]
    #[case::one_line(TorrentAdded { download: DownloadId(Uuid::from_u128(4)), name: "Dune".into(), item: None }.into(), "Added torrent Dune")]
    #[case::removed_with_files(
        MovieRemoved { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into(), delete_files: true }.into(),
        "Removed movie Dune and its files",
    )]
    #[case::one_file(FilesFound { files: vec![linked("/movies/Dune.mkv")] }.into(), "Found /movies/Dune.mkv")]
    #[case::a_line_per_file(
        FilesFound { files: vec![linked("/a.mkv"), linked("/b.mkv")] }.into(),
        "Found 2 files\n/a.mkv\n/b.mkv",
    )]
    fn describes_the_event(#[case] event: Event, #[case] expected: &str) {
        assert_eq!(event.to_string(), expected);
    }

    #[rstest]
    #[case::series_added(
        SeriesAdded { series: SeriesId(Uuid::from_u128(7)), title: "Frieren".into() }.into(),
        json!({ "type": "SeriesAdded", "series": "00000000-0000-0000-0000-000000000007", "title": "Frieren" }),
    )]
    #[case::movie_added(
        MovieAdded { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into() }.into(),
        json!({ "type": "MovieAdded", "movie": "00000000-0000-0000-0000-000000000003", "title": "Dune" }),
    )]
    #[case::series_removed(
        SeriesRemoved { series: SeriesId(Uuid::from_u128(7)), title: "Frieren".into(), delete_files: true }.into(),
        json!({ "type": "SeriesRemoved", "series": "00000000-0000-0000-0000-000000000007", "title": "Frieren", "delete_files": true }),
    )]
    #[case::movie_removed(
        MovieRemoved { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into(), delete_files: false }.into(),
        json!({ "type": "MovieRemoved", "movie": "00000000-0000-0000-0000-000000000003", "title": "Dune", "delete_files": false }),
    )]
    #[case::files_found(
        FilesFound { files: vec![LinkedFile {
            file: MediaFileId(Uuid::from_u128(5)),
            path: "/tv/Frieren/S01E01-E02.mkv".into(),
            target: FileTarget::Episodes { series: SeriesId(Uuid::from_u128(7)), span: EpisodeSpan::new(1, 1, 2).unwrap() },
        }] }.into(),
        json!({ "type": "FilesFound", "files": [{
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/tv/Frieren/S01E01-E02.mkv",
            "target": { "Episodes": {
                "series": "00000000-0000-0000-0000-000000000007",
                "span": { "season": 1, "first": 1, "last": 2 },
            } },
        }] }),
    )]
    #[case::files_imported(
        FilesImported { import: ImportId(Uuid::from_u128(9)), download: Some(DownloadId(Uuid::from_u128(4))), files: vec![LinkedFile {
            file: MediaFileId(Uuid::from_u128(5)),
            path: "/movies/Dune.mkv".into(),
            target: FileTarget::Movie(MovieId(Uuid::from_u128(3))),
        }] , sources: Vec::new() }.into(),
        json!({ "type": "FilesImported", "import": "00000000-0000-0000-0000-000000000009", "download": "00000000-0000-0000-0000-000000000004", "files": [{
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/movies/Dune.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
        }] }),
    )]
    #[case::file_deleted(
        FileDeleted {
            file: MediaFileId(Uuid::from_u128(5)),
            path: "/movies/Dune.mkv".into(),
            target: FileTarget::Movie(MovieId(Uuid::from_u128(3))),
            reason: DeleteReason::User,
        }.into(),
        json!({
            "type": "FileDeleted",
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/movies/Dune.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
            "reason": "User",
        }),
    )]
    #[case::file_renamed(
        FileRenamed {
            file: MediaFileId(Uuid::from_u128(5)),
            from: "/tv/a.mkv".into(),
            to: "/tv/A (2023)/a.mkv".into(),
            target: Some(FileTarget::Movie(MovieId(Uuid::from_u128(3)))),
        }.into(),
        json!({
            "type": "FileRenamed",
            "file": "00000000-0000-0000-0000-000000000005",
            "from": "/tv/a.mkv",
            "to": "/tv/A (2023)/a.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
        }),
    )]
    #[case::episodes_renumbered(
        EpisodesRenumbered {
            series: SeriesId(Uuid::from_u128(7)),
            files: vec![
                RenumberedFile { file: MediaFileId(Uuid::from_u128(5)), span: EpisodeSpan::new(2, 1, 1) },
                RenumberedFile { file: MediaFileId(Uuid::from_u128(6)), span: None },
            ],
        }.into(),
        json!({ "type": "EpisodesRenumbered", "series": "00000000-0000-0000-0000-000000000007", "files": [
            { "file": "00000000-0000-0000-0000-000000000005", "span": { "season": 2, "first": 1, "last": 1 } },
            { "file": "00000000-0000-0000-0000-000000000006", "span": null },
        ] }),
    )]
    #[case::import_needs_review(
        ImportNeedsReview { import: ImportId(Uuid::from_u128(9)), source: "/tv/Unknown".into() }.into(),
        json!({ "type": "ImportNeedsReview", "import": "00000000-0000-0000-0000-000000000009", "source": "/tv/Unknown" }),
    )]
    #[case::torrent_added(
        TorrentAdded {
            download: DownloadId(Uuid::from_u128(4)),
            name: "Frieren.S01.1080p".into(),
            item: Some(ItemId::Series(SeriesId(Uuid::from_u128(7)))),
        }.into(),
        json!({
            "type": "TorrentAdded",
            "download": "00000000-0000-0000-0000-000000000004",
            "name": "Frieren.S01.1080p",
            "item": { "Series": "00000000-0000-0000-0000-000000000007" },
        }),
    )]
    #[case::torrent_removed(
        TorrentRemoved { download: DownloadId(Uuid::from_u128(4)), name: "Dune.2021.1080p".into(), item: None }.into(),
        json!({ "type": "TorrentRemoved", "download": "00000000-0000-0000-0000-000000000004", "name": "Dune.2021.1080p", "item": null }),
    )]
    #[case::download_completed(
        DownloadCompleted {
            download: DownloadId(Uuid::from_u128(4)),
            name: "Dune.2021.1080p".into(),
            content_path: "/downloads/Dune.2021.1080p".into(),
            item: None,
            season: Some(2),
        }.into(),
        json!({
            "type": "DownloadCompleted",
            "download": "00000000-0000-0000-0000-000000000004",
            "name": "Dune.2021.1080p",
            "content_path": "/downloads/Dune.2021.1080p",
            "item": null,
            "season": 2,
        }),
    )]
    #[case::import_failed(
        ImportFailed { import: ImportId(Uuid::from_u128(9)), source: "/downloads/Dune".into(), reason: "disk full".into() }.into(),
        json!({ "type": "ImportFailed", "import": "00000000-0000-0000-0000-000000000009", "source": "/downloads/Dune", "reason": "disk full" }),
    )]
    #[case::settings_changed(
        SettingsChanged { key: "import.mode".into() }.into(),
        json!({ "type": "SettingsChanged", "key": "import.mode" }),
    )]
    fn stored_format_is_stable(#[case] event: Event, #[case] stored: serde_json::Value) {
        assert_eq!(serde_json::to_value(&event).unwrap(), stored);
        assert_eq!(serde_json::from_value::<Event>(stored).unwrap(), event);
    }

    #[test]
    fn events_stored_with_dropped_fields_still_read() {
        let deleted = json!({
            "type": "FileDeleted",
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/movies/Dune.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
            "reason": "User",
            "recycled": true,
        });

        assert!(matches!(
            serde_json::from_value(deleted).unwrap(),
            Event::FileDeleted(FileDeleted { reason: DeleteReason::User, .. })
        ));
    }

    #[test]
    fn events_stored_before_new_fields_read_with_defaults() {
        let deleted = json!({
            "type": "FileDeleted",
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/movies/Dune.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
            "reason": "External",
        });
        let renamed =
            json!({ "type": "FileRenamed", "file": "00000000-0000-0000-0000-000000000005", "from": "/a", "to": "/b" });
        let imported =
            json!({ "type": "FilesImported", "import": "00000000-0000-0000-0000-000000000009", "files": [] });

        assert!(matches!(serde_json::from_value(deleted).unwrap(), Event::FileDeleted(_)));
        assert!(matches!(
            serde_json::from_value(renamed).unwrap(),
            Event::FileRenamed(FileRenamed { target: None, .. })
        ));
        assert!(matches!(
            serde_json::from_value(imported).unwrap(),
            Event::FilesImported(FilesImported { download: None, .. })
        ));
    }

    fn any_id() -> impl Strategy<Value = Uuid> {
        any::<u128>().prop_map(Uuid::from_u128)
    }

    fn any_target() -> impl Strategy<Value = FileTarget> {
        prop_oneof![
            (any_id(), any::<u16>(), any::<u16>(), 0..5u16).prop_map(|(series, season, first, length)| {
                let span = EpisodeSpan::new(season, first, first.saturating_add(length)).unwrap();
                FileTarget::Episodes { series: SeriesId(series), span }
            }),
            any_id().prop_map(|movie| FileTarget::Movie(MovieId(movie))),
        ]
    }

    fn any_reason() -> impl Strategy<Value = DeleteReason> {
        prop_oneof![
            Just(DeleteReason::External),
            Just(DeleteReason::Replaced),
            Just(DeleteReason::User),
            Just(DeleteReason::ItemRemoved),
        ]
    }

    fn any_item() -> impl Strategy<Value = ItemId> {
        prop_oneof![
            any_id().prop_map(|id| ItemId::Series(SeriesId(id))),
            any_id().prop_map(|id| ItemId::Movie(MovieId(id))),
        ]
    }

    fn any_linked_file() -> impl Strategy<Value = LinkedFile> {
        (any_id(), any::<String>(), any_target()).prop_map(|(file, path, target)| LinkedFile {
            file: MediaFileId(file),
            path: path.into(),
            target,
        })
    }

    fn any_imported_from() -> impl Strategy<Value = ImportedFrom> {
        let paths = || prop::collection::vec(any::<String>(), 0..3);
        (any_id(), any::<String>(), paths(), paths()).prop_map(|(file, source, sidecars, merged)| ImportedFrom {
            file: MediaFileId(file),
            source: source.into(),
            sidecars: sidecars.into_iter().map(Into::into).collect(),
            merged: merged.into_iter().map(Into::into).collect(),
        })
    }

    fn any_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (any::<u128>(), any::<String>()).prop_map(|(id, title)| SeriesAdded {
                series: SeriesId(Uuid::from_u128(id)),
                title
            }
            .into()),
            (any::<u128>(), any::<String>()).prop_map(|(id, title)| MovieAdded {
                movie: MovieId(Uuid::from_u128(id)),
                title
            }
            .into()),
            (any::<u128>(), any::<String>(), any::<bool>()).prop_map(|(id, title, delete_files)| {
                SeriesRemoved { series: SeriesId(Uuid::from_u128(id)), title, delete_files }.into()
            }),
            (any::<u128>(), any::<String>(), any::<bool>()).prop_map(|(id, title, delete_files)| {
                MovieRemoved { movie: MovieId(Uuid::from_u128(id)), title, delete_files }.into()
            }),
            prop::collection::vec(any_linked_file(), 0..3).prop_map(|files| FilesFound { files }.into()),
            (
                any_id(),
                prop::option::of(any_id()),
                prop::collection::vec(any_linked_file(), 0..3),
                prop::collection::vec(any_imported_from(), 0..3)
            )
                .prop_map(|(import, download, files, sources)| FilesImported {
                    import: ImportId(import),
                    download: download.map(DownloadId),
                    files,
                    sources,
                }
                .into(),),
            (any_linked_file(), any_reason()).prop_map(|(linked, reason)| FileDeleted {
                file: linked.file,
                path: linked.path,
                target: linked.target,
                reason,
            }
            .into()),
            (any_id(), any::<String>(), any::<String>()).prop_map(|(import, source, reason)| Event::ImportFailed(
                ImportFailed { import: ImportId(import), source: source.into(), reason }
            )),
            (any_id(), any::<String>(), any::<String>(), proptest::option::of(any_target())).prop_map(
                |(file, from, to, target)| FileRenamed {
                    file: MediaFileId(file),
                    from: from.into(),
                    to: to.into(),
                    target
                }
                .into()
            ),
            (any_id(), any::<String>()).prop_map(|(import, source)| ImportNeedsReview {
                import: ImportId(import),
                source: source.into()
            }
            .into()),
            (any_id(), any::<String>(), proptest::option::of(any_item())).prop_map(|(download, name, item)| {
                TorrentAdded { download: DownloadId(download), name, item }.into()
            }),
            (any_id(), any::<String>(), proptest::option::of(any_item())).prop_map(|(download, name, item)| {
                TorrentRemoved { download: DownloadId(download), name, item }.into()
            }),
            (
                any_id(),
                any::<String>(),
                any::<String>(),
                proptest::option::of(any_item()),
                proptest::option::of(any::<u16>())
            )
                .prop_map(|(download, name, content_path, item, season)| DownloadCompleted {
                    download: DownloadId(download),
                    name,
                    content_path: content_path.into(),
                    item,
                    season,
                }
                .into()),
            any::<String>().prop_map(|key| SettingsChanged { key }.into()),
        ]
    }

    proptest! {
        #[test]
        fn round_trips_through_json(event in any_event()) {
            let stored = serde_json::to_string(&event).unwrap();
            prop_assert_eq!(serde_json::from_str::<Event>(&stored).unwrap(), event);
        }
    }
}
