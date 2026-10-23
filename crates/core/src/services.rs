use std::sync::Arc;

use yokoku_domain::Clock;

use crate::{
    downloads::{Downloads, ReleaseSearch},
    events::{History, QueueChanges},
    integrations::{Ratings, Rescans},
    library::{Artworks, Calendar, Library, MetadataService},
    media::{Deleter, Importer, Prober, Renamer, Reviewer, RootFolders, Scanner},
};

/// The use cases the web UI, the CLI and jobs call, wired by the composition root.
#[derive(Clone)]
pub struct Services {
    pub library: Arc<Library>,
    pub artworks: Arc<Artworks>,
    pub calendar: Arc<Calendar>,
    pub roots: Arc<RootFolders>,
    pub scanner: Arc<Scanner>,
    pub reviewer: Arc<Reviewer>,
    pub renamer: Arc<Renamer>,
    pub downloads: Arc<Downloads>,
    pub releases: Arc<ReleaseSearch>,
    pub importer: Arc<Importer>,
    pub history: Arc<History>,
    pub deleter: Arc<Deleter>,
    pub prober: Arc<Prober>,
    pub rescans: Arc<Rescans>,
    pub ratings: Arc<Ratings>,
    pub metadata: Arc<MetadataService>,
    pub clock: Arc<dyn Clock>,
    /// Wakes after this process changes downloads or imports.
    pub queue_changes: QueueChanges,
}
