//! Every event subscriber in the system and the events it handles. A subscriber's name keys its
//! stored delivery position; renaming one delivers the whole log to it again.

use std::sync::Arc;

use yokoku_config::Settings;
use yokoku_db::Database;
use yokoku_downloads::Downloads;
use yokoku_events::{
    DownloadCompleted, EpisodesRenumbered, FileDeleted, FileRenamed, FilesFound, FilesImported, MovieAdded,
    MovieRemoved, SeriesAdded, SeriesRemoved, SettingsChanged, Subscriber, Subscription,
};
use yokoku_integrations::Rescans;
use yokoku_library::{Artworks, FileTracker};
use yokoku_media::{Deleter, ImportPlanner, Prober, Scanner};

#[expect(clippy::too_many_arguments, reason = "one argument per subscriber")]
pub fn subscribers(
    db: &Arc<Database>,
    planner: &Arc<ImportPlanner>,
    deleter: &Arc<Deleter>,
    downloads: &Arc<Downloads>,
    prober: &Arc<Prober>,
    scanner: &Arc<Scanner>,
    rescans: &Arc<Rescans>,
    artworks: &Arc<Artworks>,
    settings: &Settings,
) -> Vec<Arc<dyn Subscriber>> {
    let tracker = Arc::new(FileTracker::new(db.clone(), db.clone(), db.clone()));
    [
        Subscription::new("media.scan_added").on::<SeriesAdded>(scanner.clone()).on::<MovieAdded>(scanner.clone()),
        Subscription::new("library.files")
            .on::<FilesFound>(tracker.clone())
            .on::<FilesImported>(tracker.clone())
            .on::<FileDeleted>(tracker),
        Subscription::new("library.artwork").on::<SeriesRemoved>(artworks.clone()).on::<MovieRemoved>(artworks.clone()),
        Subscription::new("media.imports").on::<DownloadCompleted>(planner.clone()),
        Subscription::new("media.removals").on::<SeriesRemoved>(deleter.clone()).on::<MovieRemoved>(deleter.clone()),
        Subscription::new("media.renumbered").on::<EpisodesRenumbered>(scanner.clone()),
        Subscription::new("downloads.imports").on::<FilesImported>(downloads.clone()),
        Subscription::new("media.probe").on::<FilesFound>(prober.clone()).on::<FilesImported>(prober.clone()),
        Subscription::new("integrations.rescans")
            .on::<FilesImported>(rescans.clone())
            .on::<FileRenamed>(rescans.clone())
            .on::<FileDeleted>(rescans.clone()),
        Subscription::new("config.settings").on::<SettingsChanged>(Arc::new(settings.clone())),
    ]
    .into_iter()
    .map(|subscription| Arc::new(subscription) as Arc<dyn Subscriber>)
    .collect()
}
