//! Every event subscriber in the system.

use std::sync::Arc;

use yokoku_db::Database;
use yokoku_domain::Clock;
use yokoku_events::Subscriber;
use yokoku_integrations::Rescans;
use yokoku_library::FileTracker;
use yokoku_media::{Deleter, ImportPlanner, ports::FileSystem};

pub fn subscribers(
    db: &Arc<Database>,
    fs: &Arc<dyn FileSystem>,
    clock: &Arc<dyn Clock>,
    deleter: &Arc<Deleter>,
    rescans: Option<&Arc<Rescans>>,
) -> Vec<Arc<dyn Subscriber>> {
    let mut subscribers: Vec<Arc<dyn Subscriber>> = vec![
        Arc::new(FileTracker::new(db.clone(), db.clone())),
        Arc::new(ImportPlanner::new(db.clone(), db.clone(), fs.clone(), clock.clone())),
        deleter.clone(),
    ];
    subscribers.extend(rescans.map(|rescans| rescans.clone() as Arc<dyn Subscriber>));
    subscribers
}
