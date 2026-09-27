//! Every event subscriber in the system and the events it handles. A subscriber's name keys its
//! stored delivery position; renaming one delivers the whole log to it again.

use std::sync::Arc;

use yokoku_db::Database;
use yokoku_downloads::Downloads;
use yokoku_events::{
    DownloadCompleted, EpisodesRenumbered, FileDeleted, FileRenamed, FilesFound, FilesImported, MovieAdded,
    MovieRemoved, SeriesAdded, SeriesRemoved, Subscriber, Subscription,
};
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
    let tracker = Arc::new(FileTracker::new(db.clone(), db.clone()));
    let mut subscriptions = vec![
        Subscription::new("media.scan_added").on::<SeriesAdded>(scanner.clone()).on::<MovieAdded>(scanner.clone()),
        Subscription::new("library.files")
            .on::<FilesFound>(tracker.clone())
            .on::<FilesImported>(tracker.clone())
            .on::<FileDeleted>(tracker),
        Subscription::new("media.imports").on::<DownloadCompleted>(planner.clone()),
        Subscription::new("media.removals").on::<SeriesRemoved>(deleter.clone()).on::<MovieRemoved>(deleter.clone()),
        Subscription::new("media.renumbered").on::<EpisodesRenumbered>(scanner.clone()),
        Subscription::new("downloads.imports").on::<FilesImported>(downloads.clone()),
        Subscription::new("media.probe").on::<FilesFound>(prober.clone()).on::<FilesImported>(prober.clone()),
    ];
    subscriptions.extend(rescans.map(|rescans| {
        Subscription::new("integrations.rescans")
            .on::<FilesImported>(rescans.clone())
            .on::<FileRenamed>(rescans.clone())
            .on::<FileDeleted>(rescans.clone())
    }));
    subscriptions.into_iter().map(|subscription| Arc::new(subscription) as Arc<dyn Subscriber>).collect()
}
