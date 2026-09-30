use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use yokoku_domain::{MovieId, SeriesId};
use yokoku_events::{Event, Handler, HandlerError, MovieAdded, SeriesAdded, SeriesRemoved, Subscription};

#[derive(Default)]
struct Recorder {
    handled: Mutex<Vec<String>>,
}

impl Recorder {
    fn record(&self, entry: String) {
        self.handled.lock().unwrap().push(entry);
    }

    fn handled(&self) -> Vec<String> {
        self.handled.lock().unwrap().clone()
    }
}

#[async_trait]
impl Handler<SeriesAdded> for Recorder {
    async fn handle(&self, event: &SeriesAdded) -> Result<(), HandlerError> {
        self.record(format!("series {}", event.title));
        Ok(())
    }
}

#[async_trait]
impl Handler<MovieAdded> for Recorder {
    async fn handle(&self, event: &MovieAdded) -> Result<(), HandlerError> {
        self.record(format!("movie {}", event.title));
        Ok(())
    }
}

struct Failing;

#[async_trait]
impl Handler<SeriesAdded> for Failing {
    async fn handle(&self, _: &SeriesAdded) -> Result<(), HandlerError> {
        Err("handler failed".into())
    }
}

fn series_added(title: &str) -> Event {
    SeriesAdded { series: SeriesId::generate(), title: title.into() }.into()
}

#[tokio::test]
async fn each_event_goes_to_the_handlers_of_its_type_in_the_order_added() {
    let first = Arc::new(Recorder::default());
    let second = Arc::new(Recorder::default());
    let subscription = Subscription::new("recorder")
        .on::<SeriesAdded>(first.clone())
        .on::<MovieAdded>(first.clone())
        .on::<SeriesAdded>(second.clone());

    subscription.handle(&series_added("Frieren")).await.unwrap();
    subscription.handle(&MovieAdded { movie: MovieId::generate(), title: "Dune".into() }.into()).await.unwrap();

    assert_eq!(first.handled(), ["series Frieren", "movie Dune"]);
    assert_eq!(second.handled(), ["series Frieren"]);
}

#[tokio::test]
async fn events_without_a_handler_are_skipped() {
    let recorder = Arc::new(Recorder::default());
    let subscription = Subscription::new("recorder").on::<SeriesAdded>(recorder.clone());

    let removed = SeriesRemoved { series: SeriesId::generate(), title: "Frieren".into(), delete_files: true };
    subscription.handle(&removed.into()).await.unwrap();

    assert!(recorder.handled().is_empty());
}

#[tokio::test]
async fn a_failing_handler_fails_the_delivery_before_later_handlers_run() {
    let recorder = Arc::new(Recorder::default());
    let subscription =
        Subscription::new("recorder").on::<SeriesAdded>(Arc::new(Failing)).on::<SeriesAdded>(recorder.clone());

    let error = subscription.handle(&series_added("Frieren")).await.unwrap_err();

    assert_eq!(error.to_string(), "handler failed");
    assert!(recorder.handled().is_empty());
}

#[tokio::test]
async fn the_subscription_is_delivered_under_its_name() {
    assert_eq!(Subscription::new("library.files").name(), "library.files");
}
