use yokoku_core::events::{Correlated, CorrelationId, EventId, EventLog, History, MovieAdded, Recorded, SeriesAdded};
use yokoku_domain::{ItemId, MovieId, SeriesId};
use yokoku_infra::db::Database;

fn ids(entries: &[Recorded]) -> Vec<i64> {
    entries.iter().map(|recorded| recorded.id.0).collect()
}

/// Events 1..=count; odd ones concern `series`, even ones a different movie each.
async fn logged(db: &Database, series: SeriesId, count: usize) {
    let correlation = CorrelationId::generate();
    let events: Vec<Correlated> = (1..=count)
        .map(|n| match n % 2 {
            1 => SeriesAdded { series, title: format!("Series {n}") }.into(),
            _ => MovieAdded { movie: MovieId::generate(), title: format!("Movie {n}") }.into(),
        })
        .map(|event| Correlated { correlation, event })
        .collect();
    EventLog::new(db.clone()).append(&events).await.unwrap();
}

#[tokio::test]
async fn history_pages_backwards_from_the_newest_event() {
    let db = Database::open_in_memory().await.unwrap();
    logged(&db, SeriesId::generate(), 5).await;
    let history = History::new(EventLog::new(db.clone()));

    let newest = history.page(None, None, 2).await.unwrap();
    let older = history.page(None, Some(newest[1].id), 10).await.unwrap();

    assert_eq!(ids(&newest), [5, 4]);
    assert_eq!(ids(&older), [3, 2, 1]);
}

#[tokio::test]
async fn history_of_an_item_skips_other_events_across_batches() {
    let db = Database::open_in_memory().await.unwrap();
    let series = SeriesId::generate();
    logged(&db, series, 1000).await;
    let history = History::new(EventLog::new(db.clone()));

    let entries = history.page(Some(ItemId::Series(series)), Some(EventId(900)), 300).await.unwrap();

    assert_eq!(entries.len(), 300);
    assert_eq!((entries[0].id, entries[299].id), (EventId(899), EventId(301)));
    assert!(entries.iter().all(|recorded| recorded.event.items() == [ItemId::Series(series)]));
    assert!(history.page(Some(ItemId::Movie(MovieId::generate())), None, 10).await.unwrap().is_empty());
}
