use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use rstest::{fixture, rstest};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use yokoku_db::Database;
use yokoku_domain::SeriesId;
use yokoku_events::{Delivery, DeliveryConfig, Event, EventId, EventLog, Failure, HandlerError, Recorded, Subscriber};

const SUBSCRIBER: &str = "recorder";
const WAIT: Duration = Duration::from_secs(5);
const NO_POLLING: Duration = Duration::from_secs(3600);

struct Recorder {
    failures_left: Mutex<HashMap<EventId, u32>>,
    handled: UnboundedSender<EventId>,
}

#[async_trait]
impl Subscriber for Recorder {
    fn name(&self) -> &'static str {
        SUBSCRIBER
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        if let Some(left @ 1..) = self.failures_left.lock().unwrap().get_mut(&recorded.id) {
            *left -= 1;
            return Err("handler failed".into());
        }
        self.handled.send(recorded.id)?;
        Ok(())
    }
}

struct Harness {
    db: Database,
    recorder: Option<Recorder>,
    handled: UnboundedReceiver<EventId>,
    shutdown: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl Harness {
    fn new(db: Database) -> Self {
        let (handled_tx, handled) = mpsc::unbounded_channel();
        Self {
            db,
            recorder: Some(Recorder { failures_left: Mutex::default(), handled: handled_tx }),
            handled,
            shutdown: CancellationToken::new(),
            task: None,
        }
    }

    fn failing(mut self, event: i64, times: u32) -> Self {
        let recorder = self.recorder.as_mut().expect("configure before start");
        recorder.failures_left.lock().unwrap().insert(EventId(event), times);
        self
    }

    fn start(&mut self, poll_interval: Duration) {
        let config = DeliveryConfig {
            poll_interval,
            max_attempts: 3,
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(100),
            ..DeliveryConfig::default()
        };
        let recorder = Arc::new(self.recorder.take().expect("started once"));
        let log = Arc::new(self.db.event_log());
        let delivery = Delivery::new(log, recorder, self.db.new_events().listen(), config);
        self.task = Some(tokio::spawn(delivery.run(self.shutdown.clone())));
    }

    async fn next_handled(&mut self) -> EventId {
        timeout(WAIT, self.handled.recv()).await.expect("an event is delivered in time").unwrap()
    }

    async fn position_reaches(&self, event: i64) {
        let log = self.db.event_log();
        timeout(WAIT, async {
            while log.last_delivered(SUBSCRIBER).await.unwrap() != Some(EventId(event)) {
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the position advances in time");
    }
}

async fn append(db: &Database, count: i64) {
    let events: Vec<_> =
        (1..=count).map(|id| Event::SeriesAdded { series: SeriesId(id), title: format!("Series {id}") }).collect();
    let tx = db.begin().await.unwrap();
    db.commit(tx, &events).await.unwrap();
}

#[fixture]
async fn harness() -> Harness {
    Harness::new(Database::open_in_memory().await.unwrap())
}

#[rstest]
#[tokio::test]
async fn delivers_events_in_order_and_advances_the_position(#[future(awt)] mut harness: Harness) {
    append(&harness.db, 3).await;
    harness.start(NO_POLLING);

    for expected in 1..=3 {
        assert_eq!(harness.next_handled().await, EventId(expected));
    }
    harness.position_reaches(3).await;
}

#[rstest]
#[tokio::test]
async fn resumes_after_the_saved_position(#[future(awt)] mut harness: Harness) {
    append(&harness.db, 3).await;
    harness.db.event_log().mark_delivered(SUBSCRIBER, EventId(2)).await.unwrap();
    harness.start(NO_POLLING);

    assert_eq!(harness.next_handled().await, EventId(3));
}

#[rstest]
#[tokio::test]
async fn retries_a_failing_handler_until_it_succeeds(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, 2);
    append(&harness.db, 1).await;
    harness.start(NO_POLLING);

    assert_eq!(harness.next_handled().await, EventId(1));
    harness.position_reaches(1).await;
    assert!(harness.db.event_log().failed_deliveries(SUBSCRIBER).await.unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn gives_up_after_max_attempts_and_moves_on(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, u32::MAX);
    append(&harness.db, 2).await;
    harness.start(NO_POLLING);

    assert_eq!(harness.next_handled().await, EventId(2));
    harness.position_reaches(2).await;
    assert_eq!(
        harness.db.event_log().failed_deliveries(SUBSCRIBER).await.unwrap(),
        [Failure { event: EventId(1), error: "handler failed".into(), attempts: 3 }]
    );
}

#[rstest]
#[tokio::test]
async fn wakes_up_when_events_are_committed(#[future(awt)] mut harness: Harness) {
    harness.start(NO_POLLING);
    sleep(Duration::from_millis(50)).await;

    append(&harness.db, 1).await;

    assert_eq!(harness.next_handled().await, EventId(1));
}

#[tokio::test]
async fn polls_for_events_committed_by_another_process() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");
    let mut harness = Harness::new(Database::open(&path).await.unwrap());
    let other_process = Database::open(&path).await.unwrap();
    harness.start(Duration::from_millis(100));
    sleep(Duration::from_millis(50)).await;

    append(&other_process, 1).await;

    assert_eq!(harness.next_handled().await, EventId(1));
}

#[rstest]
#[tokio::test]
async fn stops_on_shutdown(#[future(awt)] mut harness: Harness) {
    harness.start(NO_POLLING);

    harness.shutdown.cancel();

    let task = harness.task.take().unwrap();
    timeout(WAIT, task).await.expect("delivery stops in time").unwrap();
}
