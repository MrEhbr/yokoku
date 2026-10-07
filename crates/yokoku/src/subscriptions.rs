//! Every event subscriber in the system and the events it handles. A subscriber's name keys its
//! stored delivery position; renaming one delivers the whole log to it again.

use std::sync::Arc;

use yokoku_core::events::Subscription;
use yokoku_domain::events::{
    DownloadCompleted, EpisodesRenumbered, FileDeleted, FileRenamed, FilesFound, FilesImported, MovieAdded,
    MovieRemoved, SeriesAdded, SeriesRemoved, SettingsChanged,
};

use crate::app::App;

impl App {
    pub(crate) fn subscriptions(&self) -> Vec<Arc<Subscription>> {
        [
            Subscription::new("media.scan_added")
                .on::<SeriesAdded>(self.scanner.clone())
                .on::<MovieAdded>(self.scanner.clone()),
            Subscription::new("integrations.ratings")
                .on::<SeriesAdded>(self.ratings.clone())
                .on::<MovieAdded>(self.ratings.clone()),
            Subscription::new("library.files")
                .on::<FilesFound>(self.tracker.clone())
                .on::<FilesImported>(self.tracker.clone())
                .on::<FileDeleted>(self.tracker.clone()),
            Subscription::new("library.artwork")
                .on::<SeriesRemoved>(self.artworks.clone())
                .on::<MovieRemoved>(self.artworks.clone()),
            Subscription::new("media.imports").on::<DownloadCompleted>(self.planner.clone()),
            Subscription::new("media.removals")
                .on::<SeriesRemoved>(self.deleter.clone())
                .on::<MovieRemoved>(self.deleter.clone()),
            Subscription::new("media.renumbered").on::<EpisodesRenumbered>(self.scanner.clone()),
            Subscription::new("downloads.imports").on::<FilesImported>(self.downloads.clone()),
            Subscription::new("media.probe")
                .on::<FilesFound>(self.prober.clone())
                .on::<FilesImported>(self.prober.clone()),
            Subscription::new("integrations.rescans")
                .on::<FilesImported>(self.rescans.clone())
                .on::<FileRenamed>(self.rescans.clone())
                .on::<FileDeleted>(self.rescans.clone()),
            Subscription::new("config.settings").on::<SettingsChanged>(Arc::new(self.settings.clone())),
        ]
        .into_iter()
        .map(Arc::new)
        .collect()
    }
}
