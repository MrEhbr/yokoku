use yokoku_db::Database;
use yokoku_domain::{MovieId, SeriesId};
use yokoku_events::{
    CorrelationId, Event, EventLog, MovieAdded, Publisher, Recorded, SeriesAdded, correlation::correlate,
};
use yokoku_test_support::events::{accept_events, refuse_events};

fn series_added(title: &str) -> Event {
    SeriesAdded { series: SeriesId::generate(), title: title.into() }.into()
}

async fn setup() -> (Database, Publisher) {
    let db = Database::open_in_memory().await.unwrap();
    let publisher = Publisher::new(EventLog::new(db.clone()));
    (db, publisher)
}

async fn appended(db: &Database) -> Vec<Recorded> {
    EventLog::new(db.clone()).read_after(None, 100).await.unwrap()
}

async fn events(db: &Database) -> Vec<Event> {
    appended(db).await.into_iter().map(|recorded| recorded.event).collect()
}

#[tokio::test]
async fn published_events_are_appended_in_order() {
    let (db, publisher) = setup().await;
    let added = series_added("Frieren");
    let movie: Event = MovieAdded { movie: MovieId::generate(), title: "Dune".into() }.into();

    publisher.publish(added.clone()).await;
    publisher.publish_all(vec![movie.clone()]).await;

    assert_eq!(events(&db).await, [added, movie]);
}

#[tokio::test]
async fn publishing_nothing_appends_nothing() {
    let (db, publisher) = setup().await;

    publisher.publish_all(Vec::new()).await;

    assert!(events(&db).await.is_empty());
}

#[tokio::test]
async fn events_the_log_refuses_are_kept_and_appended_first_by_the_next_publish() {
    let (db, publisher) = setup().await;
    let (first, second) = (series_added("first"), series_added("second"));

    refuse_events(&db).await;
    publisher.publish(first.clone()).await;
    assert!(events(&db).await.is_empty());
    accept_events(&db).await;
    publisher.publish(second.clone()).await;

    assert_eq!(events(&db).await, [first, second]);
}

#[tokio::test]
async fn a_flush_from_any_clone_appends_kept_events_without_a_new_publish() {
    let (db, publisher) = setup().await;
    let first = series_added("first");
    refuse_events(&db).await;
    publisher.publish(first.clone()).await;

    accept_events(&db).await;
    publisher.clone().flush().await.unwrap();

    assert_eq!(events(&db).await, [first]);
}

#[tokio::test]
async fn a_flush_fails_and_keeps_the_events_while_the_log_refuses_them() {
    let (db, publisher) = setup().await;
    let first = series_added("first");
    refuse_events(&db).await;
    publisher.publish(first.clone()).await;

    let refused = publisher.flush().await;
    accept_events(&db).await;
    publisher.flush().await.unwrap();

    assert!(refused.is_err());
    assert_eq!(events(&db).await, [first]);
}

#[tokio::test]
async fn events_carry_the_correlation_id_they_are_published_under() {
    let (db, publisher) = setup().await;
    let correlation = CorrelationId::generate();

    correlate(correlation, publisher.publish_all(vec![series_added("a"), series_added("b")])).await;

    let correlations: Vec<_> = appended(&db).await.into_iter().map(|recorded| recorded.correlation).collect();
    assert_eq!(correlations, [Some(correlation); 2]);
}

#[tokio::test]
async fn events_published_outside_a_correlation_share_a_new_one_per_publish() {
    let (db, publisher) = setup().await;

    publisher.publish_all(vec![series_added("a"), series_added("b")]).await;
    publisher.publish(series_added("c")).await;

    let correlations: Vec<_> = appended(&db).await.into_iter().map(|recorded| recorded.correlation).collect();
    assert_eq!(correlations[0], correlations[1]);
    assert_ne!(correlations[1], correlations[2]);
}
