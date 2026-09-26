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
use yokoku_events::{
    Delivery, DeliveryConfig, EventId, EventLog, Failure, HandlerError, Recorded, SeriesAdded, Subscriber,
};

const SUBSCRIBER: &str = "recorder";
const WAIT: Duration = Duration::from_secs(5);
const NO_POLLING: Duration = Duration::from_secs(3600);
const RETRY_INTERVAL: Duration = Duration::from_millis(200);

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

    fn delivery(&mut self, poll_interval: Duration) -> Delivery {
        let config = DeliveryConfig {
            batch_size: 2,
            poll_interval,
            max_attempts: 3,
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(100),
            retry_interval: RETRY_INTERVAL,
        };
        let recorder = Arc::new(self.recorder.take().expect("started once"));
        let log = Arc::new(self.db.event_log());
        Delivery::new(log, recorder, self.db.new_events().listen(), config)
    }

    fn start(&mut self, poll_interval: Duration) {
        let delivery = self.delivery(poll_interval);
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
    let events: Vec<_> = (1..=count)
        .map(|id| SeriesAdded { series: SeriesId::generate(), title: format!("Series {id}") }.into())
        .collect();
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

#[rstest]
#[tokio::test]
async fn catching_up_delivers_what_is_logged_and_returns(#[future(awt)] mut harness: Harness) {
    append(&harness.db, 3).await;
    let delivery = harness.delivery(NO_POLLING);

    assert_eq!(delivery.catch_up().await.unwrap(), 3);
    assert_eq!(delivery.catch_up().await.unwrap(), 0);

    for expected in 1..=3 {
        assert_eq!(harness.next_handled().await, EventId(expected));
    }
    harness.position_reaches(3).await;
}

#[rstest]
#[tokio::test]
async fn catching_up_retries_events_given_up_on_earlier(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, 3);
    append(&harness.db, 2).await;
    let delivery = harness.delivery(NO_POLLING);
    delivery.catch_up().await.unwrap();
    assert_eq!(harness.next_handled().await, EventId(2));
    assert_eq!(harness.db.event_log().failed_deliveries(SUBSCRIBER).await.unwrap().len(), 1);

    delivery.catch_up().await.unwrap();

    assert_eq!(harness.next_handled().await, EventId(1));
    assert!(harness.db.event_log().failed_deliveries(SUBSCRIBER).await.unwrap().is_empty());
    assert_eq!(harness.db.event_log().last_delivered(SUBSCRIBER).await.unwrap(), Some(EventId(2)));
}

#[rstest]
#[tokio::test]
async fn a_retry_that_fails_again_counts_the_attempt_and_keeps_the_position(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, u32::MAX);
    append(&harness.db, 2).await;
    let delivery = harness.delivery(NO_POLLING);
    delivery.catch_up().await.unwrap();

    assert_eq!(delivery.retry_failed().await.unwrap(), 0);

    let log = harness.db.event_log();
    assert_eq!(
        log.failed_deliveries(SUBSCRIBER).await.unwrap(),
        [Failure { event: EventId(1), error: "handler failed".into(), attempts: 4 }]
    );
    assert_eq!(log.last_delivered(SUBSCRIBER).await.unwrap(), Some(EventId(2)));
}

#[rstest]
#[tokio::test]
async fn the_delivery_loop_retries_failed_events_on_its_interval(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, 3);
    append(&harness.db, 1).await;

    harness.start(Duration::from_millis(50));

    assert_eq!(harness.next_handled().await, EventId(1));
    timeout(WAIT, async {
        while !harness.db.event_log().failed_deliveries(SUBSCRIBER).await.unwrap().is_empty() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the failure is resolved in time");
}
