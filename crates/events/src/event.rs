use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use yokoku_domain::{DownloadId, FileTarget, ImportId, ItemId, MediaFileId, MovieId, SeriesId};

/// Stored events never change meaning; a breaking change adds a new variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    SeriesAdded {
        series: SeriesId,
        title: String,
    },
    MovieAdded {
        movie: MovieId,
        title: String,
    },
    SeriesRemoved {
        series: SeriesId,
        title: String,
        delete_files: bool,
    },
    MovieRemoved {
        movie: MovieId,
        title: String,
        delete_files: bool,
    },
    /// A scan linked files already in a root folder.
    FilesFound {
        files: Vec<LinkedFile>,
    },
    /// An approved import placed files in the library.
    FilesImported {
        import: ImportId,
        files: Vec<LinkedFile>,
    },
    FileDeleted {
        file: MediaFileId,
        path: PathBuf,
        target: FileTarget,
        reason: DeleteReason,
    },
    FileRenamed {
        file: MediaFileId,
        from: PathBuf,
        to: PathBuf,
    },
    ImportNeedsReview {
        import: ImportId,
        source: PathBuf,
    },
    ImportFailed {
        import: ImportId,
        source: PathBuf,
        reason: String,
    },
    TorrentAdded {
        download: DownloadId,
        name: String,
        item: Option<ItemId>,
    },
    /// Emitted once per download.
    DownloadCompleted {
        download: DownloadId,
        name: String,
        content_path: PathBuf,
        item: Option<ItemId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedFile {
    pub file: MediaFileId,
    pub path: PathBuf,
    pub target: FileTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteReason {
    /// The file disappeared outside the app.
    External,
    /// An imported file took its place.
    Replaced,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;
    use serde_json::json;
    use uuid::Uuid;
    use yokoku_domain::EpisodeSpan;

    use super::*;

    #[rstest]
    #[case::series_added(
        Event::SeriesAdded { series: SeriesId(Uuid::from_u128(7)), title: "Frieren".into() },
        json!({ "type": "SeriesAdded", "series": "00000000-0000-0000-0000-000000000007", "title": "Frieren" }),
    )]
    #[case::movie_added(
        Event::MovieAdded { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into() },
        json!({ "type": "MovieAdded", "movie": "00000000-0000-0000-0000-000000000003", "title": "Dune" }),
    )]
    #[case::series_removed(
        Event::SeriesRemoved { series: SeriesId(Uuid::from_u128(7)), title: "Frieren".into(), delete_files: true },
        json!({ "type": "SeriesRemoved", "series": "00000000-0000-0000-0000-000000000007", "title": "Frieren", "delete_files": true }),
    )]
    #[case::movie_removed(
        Event::MovieRemoved { movie: MovieId(Uuid::from_u128(3)), title: "Dune".into(), delete_files: false },
        json!({ "type": "MovieRemoved", "movie": "00000000-0000-0000-0000-000000000003", "title": "Dune", "delete_files": false }),
    )]
    #[case::files_found(
        Event::FilesFound { files: vec![LinkedFile {
            file: MediaFileId(Uuid::from_u128(5)),
            path: "/tv/Frieren/S01E01-E02.mkv".into(),
            target: FileTarget::Episodes { series: SeriesId(Uuid::from_u128(7)), span: EpisodeSpan::new(1, 1, 2).unwrap() },
        }] },
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
        Event::FilesImported { import: ImportId(Uuid::from_u128(9)), files: vec![LinkedFile {
            file: MediaFileId(Uuid::from_u128(5)),
            path: "/movies/Dune.mkv".into(),
            target: FileTarget::Movie(MovieId(Uuid::from_u128(3))),
        }] },
        json!({ "type": "FilesImported", "import": "00000000-0000-0000-0000-000000000009", "files": [{
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/movies/Dune.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
        }] }),
    )]
    #[case::file_deleted(
        Event::FileDeleted {
            file: MediaFileId(Uuid::from_u128(5)),
            path: "/movies/Dune.mkv".into(),
            target: FileTarget::Movie(MovieId(Uuid::from_u128(3))),
            reason: DeleteReason::External,
        },
        json!({
            "type": "FileDeleted",
            "file": "00000000-0000-0000-0000-000000000005",
            "path": "/movies/Dune.mkv",
            "target": { "Movie": "00000000-0000-0000-0000-000000000003" },
            "reason": "External",
        }),
    )]
    #[case::file_renamed(
        Event::FileRenamed { file: MediaFileId(Uuid::from_u128(5)), from: "/tv/a.mkv".into(), to: "/tv/A (2023)/a.mkv".into() },
        json!({
            "type": "FileRenamed",
            "file": "00000000-0000-0000-0000-000000000005",
            "from": "/tv/a.mkv",
            "to": "/tv/A (2023)/a.mkv",
        }),
    )]
    #[case::import_needs_review(
        Event::ImportNeedsReview { import: ImportId(Uuid::from_u128(9)), source: "/tv/Unknown".into() },
        json!({ "type": "ImportNeedsReview", "import": "00000000-0000-0000-0000-000000000009", "source": "/tv/Unknown" }),
    )]
    #[case::torrent_added(
        Event::TorrentAdded {
            download: DownloadId(Uuid::from_u128(4)),
            name: "Frieren.S01.1080p".into(),
            item: Some(ItemId::Series(SeriesId(Uuid::from_u128(7)))),
        },
        json!({
            "type": "TorrentAdded",
            "download": "00000000-0000-0000-0000-000000000004",
            "name": "Frieren.S01.1080p",
            "item": { "Series": "00000000-0000-0000-0000-000000000007" },
        }),
    )]
    #[case::download_completed(
        Event::DownloadCompleted {
            download: DownloadId(Uuid::from_u128(4)),
            name: "Dune.2021.1080p".into(),
            content_path: "/downloads/Dune.2021.1080p".into(),
            item: None,
        },
        json!({
            "type": "DownloadCompleted",
            "download": "00000000-0000-0000-0000-000000000004",
            "name": "Dune.2021.1080p",
            "content_path": "/downloads/Dune.2021.1080p",
            "item": null,
        }),
    )]
    #[case::import_failed(
        Event::ImportFailed { import: ImportId(Uuid::from_u128(9)), source: "/downloads/Dune".into(), reason: "disk full".into() },
        json!({ "type": "ImportFailed", "import": "00000000-0000-0000-0000-000000000009", "source": "/downloads/Dune", "reason": "disk full" }),
    )]
    fn stored_format_is_stable(#[case] event: Event, #[case] stored: serde_json::Value) {
        assert_eq!(serde_json::to_value(&event).unwrap(), stored);
        assert_eq!(serde_json::from_value::<Event>(stored).unwrap(), event);
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

    fn any_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (any::<u128>(), any::<String>())
                .prop_map(|(id, title)| Event::SeriesAdded { series: SeriesId(Uuid::from_u128(id)), title }),
            (any::<u128>(), any::<String>())
                .prop_map(|(id, title)| Event::MovieAdded { movie: MovieId(Uuid::from_u128(id)), title }),
            (any::<u128>(), any::<String>(), any::<bool>()).prop_map(|(id, title, delete_files)| {
                Event::SeriesRemoved { series: SeriesId(Uuid::from_u128(id)), title, delete_files }
            }),
            (any::<u128>(), any::<String>(), any::<bool>()).prop_map(|(id, title, delete_files)| {
                Event::MovieRemoved { movie: MovieId(Uuid::from_u128(id)), title, delete_files }
            }),
            prop::collection::vec(any_linked_file(), 0..3).prop_map(|files| Event::FilesFound { files }),
            (any_id(), prop::collection::vec(any_linked_file(), 0..3))
                .prop_map(|(import, files)| Event::FilesImported { import: ImportId(import), files }),
            (any_linked_file()).prop_map(|linked| Event::FileDeleted {
                file: linked.file,
                path: linked.path,
                target: linked.target,
                reason: DeleteReason::External,
            }),
            (any_id(), any::<String>(), any::<String>()).prop_map(|(import, source, reason)| Event::ImportFailed {
                import: ImportId(import),
                source: source.into(),
                reason,
            }),
            (any_id(), any::<String>(), any::<String>()).prop_map(|(file, from, to)| Event::FileRenamed {
                file: MediaFileId(file),
                from: from.into(),
                to: to.into(),
            }),
            (any_id(), any::<String>()).prop_map(|(import, source)| Event::ImportNeedsReview {
                import: ImportId(import),
                source: source.into()
            }),
            (any_id(), any::<String>(), proptest::option::of(any_item())).prop_map(|(download, name, item)| {
                Event::TorrentAdded { download: DownloadId(download), name, item }
            }),
            (any_id(), any::<String>(), any::<String>(), proptest::option::of(any_item())).prop_map(
                |(download, name, content_path, item)| Event::DownloadCompleted {
                    download: DownloadId(download),
                    name,
                    content_path: content_path.into(),
                    item,
                }
            ),
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
