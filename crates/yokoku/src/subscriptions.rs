//! Every event subscriber in the system and the events it handles. A subscriber's name keys its
//! stored delivery position; renaming one delivers the whole log to it again.

use std::sync::Arc;

use yokoku_core::events::Subscription;
use yokoku_domain::events::{
    DownloadCompleted, EpisodesRenumbered, FileDeleted, FileRenamed, FilesFound, FilesImported, MovieAdded,
    MovieRemoved, SeriesAdded, SeriesRemoved, SettingsChanged,
};

use crate::app::App;

pub fn subscribers(app: &App) -> Vec<Arc<Subscription>> {
    [
        Subscription::new("media.scan_added")
            .on::<SeriesAdded>(app.scanner.clone())
            .on::<MovieAdded>(app.scanner.clone()),
        Subscription::new("library.files")
            .on::<FilesFound>(app.tracker.clone())
            .on::<FilesImported>(app.tracker.clone())
            .on::<FileDeleted>(app.tracker.clone()),
        Subscription::new("library.artwork")
            .on::<SeriesRemoved>(app.artworks.clone())
            .on::<MovieRemoved>(app.artworks.clone()),
        Subscription::new("media.imports").on::<DownloadCompleted>(app.planner.clone()),
        Subscription::new("media.removals")
            .on::<SeriesRemoved>(app.deleter.clone())
            .on::<MovieRemoved>(app.deleter.clone()),
        Subscription::new("media.renumbered").on::<EpisodesRenumbered>(app.scanner.clone()),
        Subscription::new("downloads.imports").on::<FilesImported>(app.downloads.clone()),
        Subscription::new("media.probe").on::<FilesFound>(app.prober.clone()).on::<FilesImported>(app.prober.clone()),
        Subscription::new("integrations.rescans")
            .on::<FilesImported>(app.rescans.clone())
            .on::<FileRenamed>(app.rescans.clone())
            .on::<FileDeleted>(app.rescans.clone()),
        Subscription::new("config.settings").on::<SettingsChanged>(Arc::new(app.settings.clone())),
    ]
    .into_iter()
    .map(Arc::new)
    .collect()
}
