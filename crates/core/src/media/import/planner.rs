use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use tracing::{debug, info};
use yokoku_domain::{
    Clock, FileTarget, ImportId, ItemId, Movie, Series,
    events::{Event, ImportFailed, ImportNeedsReview},
};

use crate::{
    events::{HandlerError, Publisher, QueueChanges, Subscriber},
    media::{
        Import, ImportRow, ImportStatus, MediaError, Resolution,
        detect::{ImportPlan, ListedFile, MatchScope},
        ports::{Catalog, Changes, FileSystem, MediaRepo},
    },
};

/// Plans an import for every finished download.
pub struct ImportPlanner {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    clock: Arc<dyn Clock>,
    events: Publisher,
    changes: QueueChanges,
}

impl ImportPlanner {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        clock: Arc<dyn Clock>,
        events: Publisher,
        changes: QueueChanges,
    ) -> Self {
        Self { repo, catalog, fs, clock, events, changes }
    }

    /// The files of a folder download, or the single file of a one-file download; none yet, or a
    /// `.part` file among them, is `ContentNotReady`.
    async fn content_files(&self, content: &Path) -> Result<Vec<ListedFile>, MediaError> {
        let files = if self.fs.is_dir(content).await? {
            self.fs.files(content).await?
        } else {
            let folder = content.parent().unwrap_or(content);
            self.fs.files_in(folder).await?.into_iter().filter(|file| file.path == content).collect()
        };
        let partial = files.iter().any(|file| file.path.extension().is_some_and(|extension| extension == "part"));
        if files.is_empty() || partial {
            return Err(MediaError::ContentNotReady(content.to_owned()));
        }
        Ok(files)
    }

    /// The linked item while it is in the library; the whole library otherwise.
    async fn scope(&self, item: Option<ItemId>, season: Option<u16>) -> Result<Scope, MediaError> {
        let linked = match item {
            Some(ItemId::Series(id)) => self.catalog.series(id).await?.map(|series| Scope::Series(series, season)),
            Some(ItemId::Movie(id)) => self.catalog.movie(id).await?.map(Scope::Movie),
            None => None,
        };
        match linked {
            Some(scope) => Ok(scope),
            None => Ok(Scope::Library(self.catalog.all_series().await?, self.catalog.all_movies().await?)),
        }
    }
}

/// Detects what a finished download holds. The import is `Approved` when every file is certain and
/// free of conflicts, `NeedsReview` otherwise, and `Failed` without any video. Files without a
/// complete match start unchecked. A download that already has an import is left alone.
#[async_trait]
impl Subscriber for ImportPlanner {
    fn name(&self) -> &'static str {
        "media.imports"
    }

    async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        let Event::DownloadCompleted(completed) = event else { return Ok(()) };
        let download = completed.download;
        if self.repo.import_for_download(download).await?.is_some() {
            debug!(%download, "the download already has an import");
            return Ok(());
        }
        let content = completed.content_path.as_path();
        let base = content.parent().unwrap_or(content);
        let files: Vec<ListedFile> = self
            .content_files(content)
            .await?
            .into_iter()
            .filter_map(|file| {
                let path = file.path.strip_prefix(base).ok()?.to_owned();
                Some(ListedFile { path, size: file.size })
            })
            .collect();

        let scope = self.scope(completed.item, completed.season).await?;
        let plan = ImportPlan::new(&files, scope.match_scope());
        let linked: Vec<FileTarget> = self.repo.files().await?.into_iter().map(|file| file.target).collect();
        let takes_linked =
            |target: &Option<FileTarget>| target.is_some_and(|target| linked.iter().any(|file| file.overlaps(&target)));

        let status = if plan.rows.is_empty() {
            ImportStatus::Failed
        } else if plan.is_automatic() && !plan.rows.iter().any(|row| takes_linked(&row.target)) {
            ImportStatus::Approved
        } else {
            ImportStatus::NeedsReview
        };
        let rows = plan
            .rows
            .into_iter()
            .map(|row| ImportRow {
                path: base.join(&row.video.path),
                size: row.video.size,
                matched: row.row_match(completed.season),
                confidence: row.confidence,
                skipped: row.target.is_none(),
                resolution: Resolution::Unresolved,
            })
            .collect();
        let import = Import {
            id: ImportId::generate(),
            source: content.to_owned(),
            download: Some(download),
            status,
            error: (status == ImportStatus::Failed).then(|| MediaError::NoVideos.to_string()),
            rows,
            created_at: self.clock.now().timestamp(),
        };

        let event: Option<Event> = match status {
            ImportStatus::NeedsReview => {
                Some(ImportNeedsReview { import: import.id, source: import.source.clone() }.into())
            },
            ImportStatus::Failed => Some(
                ImportFailed {
                    import: import.id,
                    source: import.source.clone(),
                    reason: import.error.clone().unwrap_or_default(),
                }
                .into(),
            ),
            _ => None,
        };
        self.repo.save(&Changes { imports: vec![import.clone()], ..Changes::default() }).await?;
        self.changes.notify();
        info!(%download, import = %import.id, status = ?import.status, rows = import.rows.len(), "import planned");
        self.events.publish_all(event.into_iter().collect()).await;
        Ok(())
    }
}

enum Scope {
    Series(Series, Option<u16>),
    Movie(Movie),
    Library(Vec<Series>, Vec<Movie>),
}

impl Scope {
    fn match_scope(&self) -> MatchScope<'_> {
        match self {
            Self::Series(series, None) => MatchScope::Series(series),
            Self::Series(series, Some(season)) => MatchScope::SeriesSeason { series, season: *season },
            Self::Movie(movie) => MatchScope::Movie(movie),
            Self::Library(series, movies) => MatchScope::Library { series, movies },
        }
    }
}
