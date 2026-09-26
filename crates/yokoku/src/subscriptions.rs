//! Every event subscriber in the system.

use std::sync::Arc;

use yokoku_db::Database;
use yokoku_events::Subscriber;
use yokoku_library::FileTracker;

pub fn subscribers(db: &Arc<Database>) -> Vec<Arc<dyn Subscriber>> {
    vec![Arc::new(FileTracker::new(db.clone(), db.clone()))]
}
