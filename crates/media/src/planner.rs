use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use yokoku_detect::{DownloadFile, ImportPlan, Target};
use yokoku_domain::{Clock, DownloadId, FileTarget, ImportId, ItemId, Movie, Series};
use yokoku_events::{DownloadCompleted, Handler, HandlerError, ImportFailed, ImportNeedsReview};

use crate::{
    Import, ImportRow, ImportStatus, MediaError,
    ports::{Catalog, Changes, FileSystem, MediaRepo},
};

/// Plans an import for every finished download (FR-3.5, FR-4).
pub struct ImportPlanner {
    repo: Arc<dyn MediaRepo>,
    catalog: Arc<dyn Catalog>,
    fs: Arc<dyn FileSystem>,
    clock: Arc<dyn Clock>,
}

impl ImportPlanner {
    pub fn new(
        repo: Arc<dyn MediaRepo>,
        catalog: Arc<dyn Catalog>,
        fs: Arc<dyn FileSystem>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { repo, catalog, fs, clock }
    }

    /// Detects what the download holds. The import is `Approved` when every file is certain and
    /// free of conflicts, `NeedsReview` otherwise, and `Failed` without any video. A download that
    /// already has an import is left alone.
    pub async fn plan(
        &self,
        download: DownloadId,
        content: &Path,
        item: Option<ItemId>,
    ) -> Result<Option<Import>, MediaError> {
        if self.repo.import_for_download(download).await?.is_some() {
            return Ok(None);
        }
        let base = content.parent().unwrap_or(content);
        let files: Vec<DownloadFile> = self
            .content_files(content)
            .await?
            .into_iter()
            .filter_map(|file| {
                let path = file.path.strip_prefix(base).ok()?.to_owned();
                Some(DownloadFile { path, size: file.size })
            })
            .collect();

        let scope = self.scope(item).await?;
        let plan = ImportPlan::new(&files, scope.target());
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
                target: row.target,
                confidence: row.confidence,
                skipped: false,
                replace: false,
            })
            .collect();
        let import = Import {
            id: ImportId::generate(),
            source: content.to_owned(),
            download: Some(download),
            status,
            error: (status == ImportStatus::Failed).then(|| "the download holds no video files".to_owned()),
            rows,
            created_at: self.clock.now().timestamp(),
        };

        let event = match status {
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
        self.repo.save(&Changes { imports: vec![import.clone()], ..Changes::default() }, event.as_slice()).await?;
        Ok(Some(import))
    }

    /// The files of a folder download, or the single file of a one-file download.
    async fn content_files(&self, content: &Path) -> Result<Vec<DownloadFile>, MediaError> {
        if self.fs.is_dir(content).await? {
            return Ok(self.fs.files(content).await?);
        }
        let folder = content.parent().unwrap_or(content);
        Ok(self.fs.files_in(folder).await?.into_iter().filter(|file| file.path == content).collect())
    }

    /// The linked item while it is in the library; the whole library otherwise.
    async fn scope(&self, item: Option<ItemId>) -> Result<Scope, MediaError> {
        let linked = match item {
            Some(ItemId::Series(id)) => self.catalog.series(id).await?.map(Scope::Series),
            Some(ItemId::Movie(id)) => self.catalog.movie(id).await?.map(Scope::Movie),
            None => None,
        };
        match linked {
            Some(scope) => Ok(scope),
            None => Ok(Scope::Library(self.catalog.all_series().await?, self.catalog.all_movies().await?)),
        }
    }
}

enum Scope {
    Series(Series),
    Movie(Movie),
    Library(Vec<Series>, Vec<Movie>),
}

impl Scope {
    fn target(&self) -> Target<'_> {
        match self {
            Self::Series(series) => Target::Series(series),
            Self::Movie(movie) => Target::Movie(movie),
            Self::Library(series, movies) => Target::Library { series, movies },
        }
    }
}

#[async_trait]
impl Handler<DownloadCompleted> for ImportPlanner {
    async fn handle(&self, event: &DownloadCompleted) -> Result<(), HandlerError> {
        self.plan(event.download, &event.content_path, event.item).await?;
        Ok(())
    }
}
