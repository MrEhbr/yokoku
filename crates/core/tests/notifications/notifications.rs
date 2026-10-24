use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use jiff::{SignedDuration, Timestamp, civil::date};
use rstest::rstest;
use yokoku_core::{
    events::Subscriber,
    jobs::Job,
    library::ports::{MovieRepo, SeriesRepo},
    notifications::{
        NotificationSettings, Notifications,
        ports::{Notification, Notifier, NotifyError, Priority},
    },
};
use yokoku_domain::{
    DownloadId, EpisodeSpan, FileTarget, ImportId, ItemFolder, ItemId, Live, MediaFileId, MonitorPreset, Movie,
    MovieId, Releases, Series, SeriesId, SourceStatus,
    events::{DownloadCompleted, Event, FilesImported, ImportFailed, ImportNeedsReview, LinkedFile, TorrentAdded},
};
use yokoku_infra::db::Database;
use yokoku_test_support::{
    clock::TestClock,
    metadata::{movie_metadata, series_metadata},
};

const QUIET: SignedDuration = SignedDuration::from_secs(30);

/// Records what it sends; can fail a number of sends, or be unconfigured.
#[derive(Default)]
struct RecordingNotifier {
    sent: Mutex<Vec<Notification>>,
    attempts: Mutex<u32>,
    failures_left: Mutex<u32>,
    unconfigured: Mutex<bool>,
}

impl RecordingNotifier {
    fn sent(&self) -> Vec<Notification> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl Notifier for RecordingNotifier {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError> {
        *self.attempts.lock().unwrap() += 1;
        if *self.unconfigured.lock().unwrap() {
            return Err(NotifyError::NotConfigured);
        }
        let mut failures_left = self.failures_left.lock().unwrap();
        if *failures_left > 0 {
            *failures_left -= 1;
            return Err(NotifyError::Unavailable("connection refused".into()));
        }
        self.sent.lock().unwrap().push(notification.clone());
        Ok(())
    }
}

struct Setup {
    clock: Arc<TestClock>,
    notifier: Arc<RecordingNotifier>,
    notifications: Notifications,
    frieren: SeriesId,
    dune: MovieId,
}

impl Setup {
    async fn handle(&self, event: impl Into<Event>) {
        self.notifications.handle(&event.into()).await.unwrap();
    }

    /// Moves the clock past the quiet period, then sends what is due.
    async fn send_after_quiet(&self) {
        self.clock.advance(QUIET);
        self.notifications.run().await.unwrap();
    }
}

async fn setup() -> Setup {
    setup_with(NotificationSettings::default()).await
}

async fn setup_with(settings: NotificationSettings) -> Setup {
    let db = Database::open_in_memory().await.unwrap();
    let mut frieren = Series::new(
        series_metadata(209867, "Frieren", SourceStatus::Returning, &[]),
        ItemFolder::new("/tv".into(), "Frieren (2023)".into()).unwrap(),
        MonitorPreset::All,
        date(2023, 9, 29),
        Timestamp::UNIX_EPOCH,
    );
    let mut dune = Movie::new(
        movie_metadata(438631, "Dune", Releases::default()),
        ItemFolder::new("/movies".into(), "Dune (2021)".into()).unwrap(),
        true,
        Timestamp::UNIX_EPOCH,
    );
    SeriesRepo::save(&db, &mut frieren).await.unwrap();
    MovieRepo::save(&db, &mut dune).await.unwrap();
    let clock = Arc::new(TestClock::default());
    let notifier = Arc::new(RecordingNotifier::default());
    let notifications =
        Notifications::new(notifier.clone(), Arc::new(db.clone()), Arc::new(db), Live::fixed(settings), clock.clone());
    Setup { clock, notifier, notifications, frieren: frieren.id, dune: dune.id }
}

fn imported(files: Vec<LinkedFile>) -> FilesImported {
    FilesImported { import: ImportId::generate(), download: Some(DownloadId::generate()), files, sources: Vec::new() }
}

/// An import of `episodes` of season 1 of `series`, a file each.
fn episodes(series: SeriesId, episodes: impl IntoIterator<Item = u16>) -> FilesImported {
    imported(
        episodes
            .into_iter()
            .map(|episode| LinkedFile {
                file: MediaFileId::generate(),
                path: format!("/tv/Frieren (2023)/Season 01/Frieren (2023) - S01E{episode:02}.mkv").into(),
                target: FileTarget::Episodes { series, span: EpisodeSpan::new(1, episode, episode).unwrap() },
            })
            .collect(),
    )
}

fn notification(title: &str, message: &str, priority: Priority) -> Notification {
    Notification { title: title.to_owned(), message: message.to_owned(), priority }
}

#[tokio::test]
async fn a_season_imported_at_once_is_one_notification() {
    let setup = setup().await;

    setup.handle(episodes(setup.frieren, 1..=3)).await;
    setup.send_after_quiet().await;

    assert_eq!(
        setup.notifier.sent(),
        [notification("Frieren (2023)", "Imported 3 files\nS01E01\nS01E02\nS01E03", Priority::Normal)]
    );
}

#[tokio::test]
async fn imports_of_one_series_merge_until_none_arrives_for_the_quiet_period() {
    let setup = setup().await;
    let wait = |seconds| setup.clock.advance(SignedDuration::from_secs(seconds));

    setup.handle(episodes(setup.frieren, [1])).await;
    wait(20);
    setup.notifications.run().await.unwrap();
    setup.handle(episodes(setup.frieren, [2])).await;
    wait(20);
    setup.notifications.run().await.unwrap();
    let before_quiet = setup.notifier.sent();
    wait(10);
    setup.notifications.run().await.unwrap();

    assert!(before_quiet.is_empty(), "{before_quiet:?}");
    assert_eq!(
        setup.notifier.sent(),
        [notification("Frieren (2023)", "Imported 2 files\nS01E01\nS01E02", Priority::Normal)]
    );
}

#[tokio::test]
async fn each_kind_about_each_item_is_its_own_notification_oldest_first() {
    let setup = setup().await;
    let movie = LinkedFile {
        file: MediaFileId::generate(),
        path: "/movies/Dune (2021)/Dune (2021).mkv".into(),
        target: FileTarget::Movie(setup.dune),
    };

    setup.handle(episodes(setup.frieren, [1])).await;
    setup
        .handle(TorrentAdded {
            download: DownloadId::generate(),
            name: "Frieren S01E02 1080p".into(),
            item: Some(ItemId::Series(setup.frieren)),
        })
        .await;
    setup.handle(imported(vec![movie])).await;
    setup.send_after_quiet().await;

    assert_eq!(
        setup.notifier.sent(),
        [
            notification("Frieren (2023)", "Imported S01E01", Priority::Normal),
            notification("Frieren (2023)", "Grabbed Frieren S01E02 1080p", Priority::Normal),
            notification("Dune", "Imported Dune (2021).mkv", Priority::Normal),
        ]
    );
}

#[rstest]
#[case::needs_review(
    ImportNeedsReview { import: ImportId::generate(), source: "/downloads/Frieren.S01E05.mkv".into() }.into(),
    notification("Import needs review", "Frieren.S01E05.mkv", Priority::High)
)]
#[case::failed(
    ImportFailed {
        import: ImportId::generate(),
        source: "/downloads/Frieren.S01E05.mkv".into(),
        reason: "no space left on device".into(),
    }
    .into(),
    notification("Import failed", "Frieren.S01E05.mkv: no space left on device", Priority::High)
)]
#[case::grabbed(
    TorrentAdded { download: DownloadId::generate(), name: "Frieren S01 1080p".into(), item: None }.into(),
    notification("Grabbed", "Grabbed Frieren S01 1080p", Priority::Normal)
)]
#[case::downloaded(
    DownloadCompleted {
        download: DownloadId::generate(),
        name: "Frieren S01 1080p".into(),
        content_path: "/downloads/Frieren S01 1080p".into(),
        item: None,
        season: None,
    }
    .into(),
    notification("Downloaded", "Downloaded Frieren S01 1080p", Priority::Normal)
)]
#[case::removed_item(
    TorrentAdded {
        download: DownloadId::generate(),
        name: "Frieren S01 1080p".into(),
        item: Some(ItemId::Series(SeriesId::generate())),
    }
    .into(),
    notification("Grabbed", "Grabbed Frieren S01 1080p", Priority::Normal)
)]
#[tokio::test]
async fn an_event_about_no_item_in_the_library_is_titled_by_its_kind(
    #[case] event: Event,
    #[case] expected: Notification,
) {
    let setup = setup().await;

    setup.handle(event).await;
    setup.send_after_quiet().await;

    assert_eq!(setup.notifier.sent(), [expected]);
}

#[tokio::test]
async fn a_switched_off_kind_does_not_notify() {
    let setup = setup_with(NotificationSettings { imported: false, ..NotificationSettings::default() }).await;

    setup.handle(episodes(setup.frieren, [1])).await;
    setup.handle(TorrentAdded { download: DownloadId::generate(), name: "Frieren S01E02".into(), item: None }).await;
    setup.send_after_quiet().await;

    assert_eq!(setup.notifier.sent(), [notification("Grabbed", "Grabbed Frieren S01E02", Priority::Normal)]);
}

#[tokio::test]
async fn lines_past_ten_are_counted() {
    let setup = setup().await;

    setup.handle(episodes(setup.frieren, 1..=12)).await;
    setup.send_after_quiet().await;

    let sent = setup.notifier.sent();
    let lines: Vec<&str> = sent[0].message.lines().collect();
    assert_eq!(lines.len(), 12, "{lines:?}");
    assert_eq!(lines[0], "Imported 12 files");
    assert_eq!(lines[10], "S01E10");
    assert_eq!(lines[11], "…and 2 more");
}

#[tokio::test]
async fn a_failed_send_is_tried_again_with_what_arrived_meanwhile() {
    let setup = setup().await;
    *setup.notifier.failures_left.lock().unwrap() = 1;

    setup.handle(episodes(setup.frieren, [1])).await;
    setup.clock.advance(QUIET);
    let failed = setup.notifications.run().await;
    setup.handle(episodes(setup.frieren, [2])).await;
    setup.send_after_quiet().await;

    assert!(failed.is_err());
    assert_eq!(
        setup.notifier.sent(),
        [notification("Frieren (2023)", "Imported 2 files\nS01E01\nS01E02", Priority::Normal)]
    );
}

#[tokio::test]
async fn a_notification_is_dropped_after_five_failed_sends() {
    let setup = setup().await;
    *setup.notifier.failures_left.lock().unwrap() = u32::MAX;

    setup.handle(episodes(setup.frieren, [1])).await;
    for _ in 0..6 {
        setup.clock.advance(QUIET);
        _ = setup.notifications.run().await;
    }

    assert_eq!(*setup.notifier.attempts.lock().unwrap(), 5);
}

#[tokio::test]
async fn without_a_notification_service_waiting_notifications_are_dropped() {
    let setup = setup().await;
    *setup.notifier.unconfigured.lock().unwrap() = true;

    setup.handle(episodes(setup.frieren, [1])).await;
    setup.send_after_quiet().await;
    *setup.notifier.unconfigured.lock().unwrap() = false;
    setup.send_after_quiet().await;

    assert!(setup.notifier.sent().is_empty());
}
