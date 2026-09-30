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
use yokoku_core::events::{
    Correlated, Delivery, DeliveryConfig, DeliveryFailure, EventId, EventLog, Handler, HandlerError, Subscription,
    correlation,
};
use yokoku_domain::{CorrelationId, SeriesId, events::SeriesAdded};
use yokoku_infra::db::Database;

const SUBSCRIBER: &str = "recorder";
const WAIT: Duration = Duration::from_secs(5);
const NO_POLLING: Duration = Duration::from_secs(3600);
const RETRY_INTERVAL: Duration = Duration::from_millis(200);

struct Recorder {
    failures_left: Mutex<HashMap<EventId, u32>>,
    handled: UnboundedSender<EventId>,
}

/// Takes event n from its title, `Series n`.
#[async_trait]
impl Handler<SeriesAdded> for Recorder {
    async fn handle(&self, event: &SeriesAdded) -> Result<(), HandlerError> {
        let id = EventId(event.title.trim_start_matches("Series ").parse()?);
        if let Some(left @ 1..) = self.failures_left.lock().unwrap().get_mut(&id) {
            *left -= 1;
            return Err("handler failed".into());
        }
        self.handled.send(id)?;
        Ok(())
    }
}

struct Harness {
    log: EventLog,
    recorder: Option<Recorder>,
    handled: UnboundedReceiver<EventId>,
    shutdown: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl Harness {
    fn new(db: Database) -> Self {
        let (handled_tx, handled) = mpsc::unbounded_channel();
        Self {
            log: EventLog::new(db.clone()),
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
        Delivery::new(self.log.clone(), Arc::new(Subscription::new(SUBSCRIBER).on::<SeriesAdded>(recorder)), config)
    }

    fn start(&mut self, poll_interval: Duration) {
        let delivery = self.delivery(poll_interval);
        self.task = Some(tokio::spawn(delivery.run(self.shutdown.clone())));
    }

    async fn next_handled(&mut self) -> EventId {
        timeout(WAIT, self.handled.recv()).await.expect("an event is delivered in time").unwrap()
    }

    async fn position_reaches(&self, event: i64) {
        let log = &self.log;
        timeout(WAIT, async {
            while log.last_delivered(SUBSCRIBER).await.unwrap() != Some(EventId(event)) {
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the position advances in time");
    }
}

fn series_added(title: &str) -> Correlated {
    let event = SeriesAdded { series: SeriesId::generate(), title: title.into() }.into();
    Correlated { correlation: CorrelationId::generate(), event }
}

async fn append(log: &EventLog, count: i64) {
    let events: Vec<_> = (1..=count).map(|id| series_added(&format!("Series {id}"))).collect();
    log.append(&events).await.unwrap();
}

async fn failures(log: &EventLog) -> Vec<DeliveryFailure> {
    log.failed(SUBSCRIBER).await.unwrap().into_iter().map(|(_, failure)| failure).collect()
}

#[fixture]
async fn harness() -> Harness {
    Harness::new(Database::open_in_memory().await.unwrap())
}

#[rstest]
#[tokio::test]
async fn delivers_events_in_order_and_advances_the_position(#[future(awt)] mut harness: Harness) {
    append(&harness.log, 3).await;
    harness.start(NO_POLLING);

    for expected in 1..=3 {
        assert_eq!(harness.next_handled().await, EventId(expected));
    }
    harness.position_reaches(3).await;
}

#[rstest]
#[tokio::test]
async fn resumes_after_the_saved_position(#[future(awt)] mut harness: Harness) {
    append(&harness.log, 3).await;
    harness.log.mark_delivered(SUBSCRIBER, EventId(2)).await.unwrap();
    harness.start(NO_POLLING);

    assert_eq!(harness.next_handled().await, EventId(3));
}

#[rstest]
#[tokio::test]
async fn retries_a_failing_handler_until_it_succeeds(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, 2);
    append(&harness.log, 1).await;
    harness.start(NO_POLLING);

    assert_eq!(harness.next_handled().await, EventId(1));
    harness.position_reaches(1).await;
    assert!(failures(&harness.log).await.is_empty());
}

#[rstest]
#[tokio::test]
async fn gives_up_after_max_attempts_and_moves_on(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, u32::MAX);
    append(&harness.log, 2).await;
    harness.start(NO_POLLING);

    assert_eq!(harness.next_handled().await, EventId(2));
    harness.position_reaches(2).await;
    assert_eq!(
        failures(&harness.log).await,
        [DeliveryFailure { event: EventId(1), error: "handler failed".into(), attempts: 3 }]
    );
}

#[rstest]
#[tokio::test]
async fn wakes_up_when_events_are_committed(#[future(awt)] mut harness: Harness) {
    harness.start(NO_POLLING);
    sleep(Duration::from_millis(50)).await;

    append(&harness.log, 1).await;

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

    append(&EventLog::new(other_process.clone()), 1).await;

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
async fn a_retry_that_fails_again_counts_the_attempt_and_keeps_the_position(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, u32::MAX);
    append(&harness.log, 2).await;

    harness.start(Duration::from_millis(50));

    assert_eq!(harness.next_handled().await, EventId(2));
    let log = &harness.log;
    timeout(WAIT, async {
        while failures(log).await.first().is_none_or(|failure| failure.attempts < 4) {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the failure is retried in time");
    assert_eq!(
        failures(log).await[0],
        DeliveryFailure { event: EventId(1), error: "handler failed".into(), attempts: 4 }
    );
    assert_eq!(log.last_delivered(SUBSCRIBER).await.unwrap(), Some(EventId(2)));
}

#[rstest]
#[tokio::test]
async fn the_delivery_loop_retries_failed_events_on_its_interval(#[future(awt)] harness: Harness) {
    let mut harness = harness.failing(1, 3);
    append(&harness.log, 2).await;

    harness.start(Duration::from_millis(50));

    assert_eq!(harness.next_handled().await, EventId(2));
    assert_eq!(harness.next_handled().await, EventId(1));
    timeout(WAIT, async {
        while !failures(&harness.log).await.is_empty() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the failure is resolved in time");
    assert_eq!(harness.log.last_delivered(SUBSCRIBER).await.unwrap(), Some(EventId(2)));
}

#[rstest]
#[tokio::test]
async fn wakes_up_when_events_are_appended(#[future(awt)] mut harness: Harness) {
    harness.start(NO_POLLING);
    sleep(Duration::from_millis(50)).await;

    harness.log.append(&[series_added("Series 1")]).await.unwrap();

    assert_eq!(harness.next_handled().await, EventId(1));
}

/// Records the current correlation id of each event it handles.
#[derive(Default)]
struct CorrelationRecorder(Mutex<Vec<Option<CorrelationId>>>);

#[async_trait]
impl Handler<SeriesAdded> for CorrelationRecorder {
    async fn handle(&self, _: &SeriesAdded) -> Result<(), HandlerError> {
        self.0.lock().unwrap().push(correlation::current());
        Ok(())
    }
}

#[tokio::test]
async fn handlers_run_under_the_correlation_id_of_their_event() {
    let db = Database::open_in_memory().await.unwrap();
    let event = series_added("Series 1");
    let log = EventLog::new(db.clone());
    log.append(std::slice::from_ref(&event)).await.unwrap();
    let recorder = Arc::new(CorrelationRecorder::default());
    let delivery = Delivery::new(
        log,
        Arc::new(Subscription::new("correlations").on::<SeriesAdded>(recorder.clone())),
        DeliveryConfig::default(),
    );
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(delivery.run(shutdown.clone()));

    timeout(WAIT, async {
        while recorder.0.lock().unwrap().is_empty() {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the event is delivered in time");
    shutdown.cancel();
    task.await.unwrap();
    assert_eq!(*recorder.0.lock().unwrap(), [Some(event.correlation)]);
}
