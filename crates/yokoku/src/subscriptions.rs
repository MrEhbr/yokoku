//! Every event subscriber in the system.

use std::sync::Arc;

use yokoku_db::Database;
use yokoku_downloads::Downloads;
use yokoku_events::Subscriber;
use yokoku_integrations::Rescans;
use yokoku_library::FileTracker;
use yokoku_media::{Deleter, ImportPlanner, Prober, Scanner};

pub fn subscribers(
    db: &Arc<Database>,
    planner: &Arc<ImportPlanner>,
    deleter: &Arc<Deleter>,
    downloads: &Arc<Downloads>,
    prober: &Arc<Prober>,
    scanner: &Arc<Scanner>,
    rescans: Option<&Arc<Rescans>>,
) -> Vec<Arc<dyn Subscriber>> {
    let mut subscribers: Vec<Arc<dyn Subscriber>> = vec![
        scanner.clone(),
        Arc::new(FileTracker::new(db.clone(), db.clone())),
        planner.clone(),
        deleter.clone(),
        downloads.clone(),
        prober.clone(),
    ];
    subscribers.extend(rescans.map(|rescans| rescans.clone() as Arc<dyn Subscriber>));
    subscribers
}
