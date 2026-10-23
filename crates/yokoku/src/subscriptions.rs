//! Every event subscriber in the system.

use std::sync::Arc;

use yokoku_core::events::Subscriber;

use crate::app::App;

impl App {
    pub(crate) fn subscriptions(&self) -> Vec<Arc<dyn Subscriber>> {
        vec![
            self.scanner.clone(),
            self.ratings.clone(),
            self.tracker.clone(),
            self.artworks.clone(),
            self.planner.clone(),
            self.deleter.clone(),
            self.downloads.clone(),
            self.prober.clone(),
            self.rescans.clone(),
            Arc::new(self.settings.clone()),
        ]
    }
}
