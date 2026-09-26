//! Every event subscriber in the system.

use std::sync::Arc;

use yokoku_db::Database;
use yokoku_domain::Clock;
use yokoku_events::Subscriber;
use yokoku_library::FileTracker;
use yokoku_media::{ImportPlanner, ports::FileSystem};

pub fn subscribers(db: &Arc<Database>, fs: &Arc<dyn FileSystem>, clock: &Arc<dyn Clock>) -> Vec<Arc<dyn Subscriber>> {
    vec![
        Arc::new(FileTracker::new(db.clone(), db.clone())),
        Arc::new(ImportPlanner::new(db.clone(), db.clone(), fs.clone(), clock.clone())),
    ]
}
