use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use jiff::{SignedDuration, Timestamp, Zoned, tz::TimeZone};
use yokoku_db::Database;
use yokoku_domain::{Clock, MediaFileId, MovieId};
use yokoku_events::{DeleteReason, EventKind, FileDeleted, FileRenamed, Handler};
use yokoku_integrations::{
    Rescans,
    ports::{MediaServer, MediaServerError, RescanStore},
};

const QUIET: SignedDuration = SignedDuration::from_secs(30);

struct TestClock(Mutex<Timestamp>);

impl TestClock {
    fn advance(&self, by: SignedDuration) {
        let mut now = self.0.lock().unwrap();
        *now = now.checked_add(by).unwrap();
    }
}

impl Clock for TestClock {
    fn now(&self) -> Zoned {
        self.0.lock().unwrap().to_zoned(TimeZone::UTC)
    }
}

/// Counts refreshes; can fail, or record another request while refreshing.
#[derive(Default)]
struct RecordingServer {
    refreshes: Mutex<u32>,
    failing: Mutex<bool>,
    request_during_refresh: Mutex<Option<(Database, Timestamp)>>,
}

#[async_trait]
impl MediaServer for RecordingServer {
    async fn version(&self) -> Result<String, MediaServerError> {
        Ok("Jellyfin 10.10.7".into())
    }

    async fn refresh_library(&self) -> Result<(), MediaServerError> {
        if *self.failing.lock().unwrap() {
            return Err(MediaServerError::Unavailable("connection refused".into()));
        }
        *self.refreshes.lock().unwrap() += 1;
        let request = self.request_during_refresh.lock().unwrap().take();
        if let Some((db, at)) = request {
            db.request(at).await.unwrap();
        }
        Ok(())
    }
}

struct Setup {
    db: Database,
    clock: Arc<TestClock>,
    server: Arc<RecordingServer>,
    rescans: Rescans,
}

async fn setup() -> Setup {
    let db = Database::open_in_memory().await.unwrap();
    let clock = Arc::new(TestClock(Mutex::new("2026-09-26T12:00:00.250Z".parse().unwrap())));
    let server = Arc::new(RecordingServer::default());
    let rescans = Rescans::new(Arc::new(db.clone()), server.clone(), clock.clone());
    Setup { db, clock, server, rescans }
}

impl Setup {
    async fn handle<E: EventKind>(&self, event: E)
    where
        Rescans: Handler<E>,
    {
        self.rescans.handle(&event).await.unwrap();
    }

    fn refreshes(&self) -> u32 {
        *self.server.refreshes.lock().unwrap()
    }
}

fn deleted() -> FileDeleted {
    FileDeleted {
        file: MediaFileId::generate(),
        path: "/movies/Dune.mkv".into(),
        target: yokoku_domain::FileTarget::Movie(MovieId::generate()),
        reason: DeleteReason::User,
    }
}

#[tokio::test]
async fn nothing_pending_means_no_rescan() {
    let setup = setup().await;

    assert!(!setup.rescans.run_due(QUIET).await.unwrap());
    assert_eq!(setup.refreshes(), 0);
}

#[tokio::test]
async fn a_burst_of_changes_rescans_once_after_a_quiet_period() {
    let setup = setup().await;
    for _ in 0..3 {
        setup.handle(deleted()).await;
        setup.clock.advance(SignedDuration::from_secs(10));
    }

    let during = setup.rescans.run_due(QUIET).await.unwrap();
    setup.clock.advance(SignedDuration::from_secs(21));
    let after = setup.rescans.run_due(QUIET).await.unwrap();
    let again = setup.rescans.run_due(QUIET).await.unwrap();

    assert_eq!((during, after, again), (false, true, false));
    assert_eq!(setup.refreshes(), 1);
    assert_eq!(setup.db.requested_at().await.unwrap(), None);
}

#[tokio::test]
async fn a_change_during_the_rescan_stays_pending() {
    let setup = setup().await;
    setup.handle(deleted()).await;
    setup.clock.advance(QUIET);
    let later = setup.clock.now().timestamp();
    *setup.server.request_during_refresh.lock().unwrap() = Some((setup.db.clone(), later));

    assert!(setup.rescans.run_due(QUIET).await.unwrap());

    assert_eq!(
        setup.db.requested_at().await.unwrap(),
        Some(Timestamp::from_millisecond(later.as_millisecond()).unwrap())
    );
}

#[tokio::test]
async fn a_failed_rescan_stays_pending() {
    let setup = setup().await;
    setup.handle(deleted()).await;
    setup.clock.advance(QUIET);
    *setup.server.failing.lock().unwrap() = true;

    assert!(setup.rescans.run_due(QUIET).await.is_err());

    assert!(setup.db.requested_at().await.unwrap().is_some());
}

#[tokio::test]
async fn a_renamed_file_asks_for_a_rescan() {
    let setup = setup().await;

    setup.handle(FileRenamed { file: MediaFileId::generate(), from: "/a".into(), to: "/b".into(), target: None }).await;

    assert!(setup.db.requested_at().await.unwrap().is_some());
}

#[tokio::test]
async fn rescanning_now_ignores_the_quiet_period() {
    let setup = setup().await;
    setup.handle(deleted()).await;

    setup.rescans.run_now().await.unwrap();

    assert_eq!(setup.refreshes(), 1);
    assert_eq!(setup.db.requested_at().await.unwrap(), None);
}
