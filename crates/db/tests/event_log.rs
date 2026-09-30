use jiff::{SignedDuration, Timestamp};
use rstest::rstest;
use yokoku_db::Database;
use yokoku_domain::{MovieId, SeriesId};
use yokoku_events::{
    Correlated, CorrelationId, DeliveryFailure, Event, EventId, EventLog, MovieAdded, Recorded, SeriesAdded,
};

mod support;

use support::db;

fn series_added(id: i64) -> Event {
    SeriesAdded { series: SeriesId::generate(), title: format!("Series {id}") }.into()
}

fn ids(recorded: &[Recorded]) -> Vec<i64> {
    recorded.iter().map(|recorded| recorded.id.0).collect()
}

/// Appends `events`, each under a new correlation id.
async fn append(db: &Database, events: &[Event]) {
    let events: Vec<Correlated> = events
        .iter()
        .map(|event| Correlated { correlation: CorrelationId::generate(), event: event.clone() })
        .collect();
    db.event_log().append(&events).await.unwrap();
}

#[rstest]
#[tokio::test]
async fn appended_events_are_read_in_order(#[future(awt)] db: Database) {
    let events = [series_added(1), MovieAdded { movie: MovieId::generate(), title: "Dune".into() }.into()];

    append(&db, &events).await;

    let recorded = db.event_log().read_after(None, 10).await.unwrap();
    assert_eq!(ids(&recorded), [1, 2]);
    assert_eq!(recorded.iter().map(|recorded| recorded.event.clone()).collect::<Vec<_>>(), events);
}

#[rstest]
#[tokio::test]
async fn records_when_events_occurred(#[future(awt)] db: Database) {
    append(&db, &[series_added(1)]).await;

    let recorded = db.event_log().read_after(None, 1).await.unwrap();
    let age = Timestamp::now().duration_since(recorded[0].occurred_at);
    assert!(age >= SignedDuration::ZERO && age < SignedDuration::from_secs(60), "age was {age}");
}

#[rstest]
#[case::from_the_start(None, 10, vec![1, 2, 3, 4])]
#[case::after_a_position(Some(2), 10, vec![3, 4])]
#[case::limited(None, 2, vec![1, 2])]
#[case::after_the_end(Some(4), 10, vec![])]
#[tokio::test]
async fn read_after_returns_later_events_up_to_the_limit(
    #[future(awt)] db: Database,
    #[case] after: Option<i64>,
    #[case] limit: u32,
    #[case] expected: Vec<i64>,
) {
    append(&db, &[series_added(1), series_added(2), series_added(3), series_added(4)]).await;

    let recorded = db.event_log().read_after(after.map(EventId), limit).await.unwrap();

    assert_eq!(ids(&recorded), expected);
}

#[rstest]
#[case::from_the_end(None, 10, vec![4, 3, 2, 1])]
#[case::before_a_position(Some(3), 10, vec![2, 1])]
#[case::limited(None, 2, vec![4, 3])]
#[case::before_the_start(Some(1), 10, vec![])]
#[tokio::test]
async fn read_before_returns_earlier_events_newest_first_up_to_the_limit(
    #[future(awt)] db: Database,
    #[case] before: Option<i64>,
    #[case] limit: u32,
    #[case] expected: Vec<i64>,
) {
    append(&db, &[series_added(1), series_added(2), series_added(3), series_added(4)]).await;

    let recorded = db.event_log().read_before(before.map(EventId), limit).await.unwrap();

    assert_eq!(ids(&recorded), expected);
}

#[rstest]
#[tokio::test]
async fn keeps_a_position_per_subscriber(#[future(awt)] db: Database) {
    append(&db, &[series_added(1), series_added(2)]).await;
    let log = db.event_log();

    assert_eq!(log.last_delivered("a").await.unwrap(), None);
    log.mark_delivered("a", EventId(1)).await.unwrap();
    log.mark_delivered("a", EventId(2)).await.unwrap();
    log.mark_delivered("b", EventId(1)).await.unwrap();

    assert_eq!(log.last_delivered("a").await.unwrap(), Some(EventId(2)));
    assert_eq!(log.last_delivered("b").await.unwrap(), Some(EventId(1)));
}

#[rstest]
#[tokio::test]
async fn give_up_records_the_failure_and_advances(#[future(awt)] db: Database) {
    append(&db, &[series_added(1)]).await;
    let log = db.event_log();
    let failure = DeliveryFailure { event: EventId(1), error: "boom".into(), attempts: 5 };

    log.give_up("a", &failure).await.unwrap();
    log.give_up("a", &failure).await.unwrap();

    assert_eq!(log.last_delivered("a").await.unwrap(), Some(EventId(1)));
    assert_eq!(log.failed_deliveries("a").await.unwrap(), [failure]);
}

#[rstest]
#[tokio::test]
async fn rejects_a_position_for_an_unknown_event(#[future(awt)] db: Database) {
    assert!(db.event_log().mark_delivered("a", EventId(42)).await.is_err());
}

#[tokio::test]
async fn events_and_positions_survive_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yokoku.db");

    let db = Database::open(&path).await.unwrap();
    append(&db, &[series_added(1)]).await;
    db.event_log().mark_delivered("a", EventId(1)).await.unwrap();
    drop(db);

    let db = Database::open(&path).await.unwrap();
    let log = db.event_log();
    assert_eq!(ids(&log.read_after(None, 10).await.unwrap()), [1]);
    assert_eq!(log.last_delivered("a").await.unwrap(), Some(EventId(1)));
}

#[rstest]
#[tokio::test]
async fn failed_events_are_listed_updated_and_resolved_without_moving_the_position(#[future(awt)] db: Database) {
    append(&db, &[series_added(1), series_added(2)]).await;
    let log = db.event_log();
    log.give_up("files", &DeliveryFailure { event: EventId(1), error: "disk full".into(), attempts: 3 }).await.unwrap();
    log.mark_delivered("files", EventId(2)).await.unwrap();

    log.record_failure("files", &DeliveryFailure { event: EventId(1), error: "still full".into(), attempts: 4 })
        .await
        .unwrap();
    let failed = log.failed("files").await.unwrap();

    assert_eq!(failed.len(), 1);
    assert_eq!((failed[0].0.id, &failed[0].1.error, failed[0].1.attempts), (EventId(1), &"still full".to_owned(), 4));
    assert!(log.failed("other").await.unwrap().is_empty());
    assert_eq!(log.last_delivered("files").await.unwrap(), Some(EventId(2)));

    log.resolve("files", EventId(1)).await.unwrap();

    assert!(log.failed("files").await.unwrap().is_empty());
    assert_eq!(log.last_delivered("files").await.unwrap(), Some(EventId(2)));
}

#[rstest]
#[tokio::test]
async fn append_adds_events_in_order_after_those_already_logged(#[future(awt)] db: Database) {
    append(&db, &[series_added(1)]).await;
    let appended = [series_added(2), MovieAdded { movie: MovieId::generate(), title: "Dune".into() }.into()];

    append(&db, &appended).await;

    let recorded = db.event_log().read_after(Some(EventId(1)), 10).await.unwrap();
    assert_eq!(ids(&recorded), [2, 3]);
    assert_eq!(recorded.into_iter().map(|recorded| recorded.event).collect::<Vec<_>>(), appended);
}

#[rstest]
#[tokio::test]
async fn events_are_read_with_their_correlation_id(#[future(awt)] db: Database) {
    let correlation = CorrelationId::generate();
    let events =
        [Correlated { correlation, event: series_added(1) }, Correlated { correlation, event: series_added(2) }];

    db.event_log().append(&events).await.unwrap();

    let recorded = db.event_log().read_after(None, 10).await.unwrap();
    assert_eq!(recorded.iter().map(|recorded| recorded.correlation).collect::<Vec<_>>(), [Some(correlation); 2]);
}
