use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{FileTarget, MediaFileId};
use yokoku_events::{FileDeleted, FilesFound, FilesImported, HandlerError, LinkedFile, Recorded, Subscriber};

use crate::{
    LibraryError,
    ports::{MovieRepo, SeriesRepo},
    retry,
};

/// Keeps the file id of each episode and movie in step with media's file events.
pub struct FileTracker {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
}

impl FileTracker {
    pub fn new(series: Arc<dyn SeriesRepo>, movies: Arc<dyn MovieRepo>) -> Self {
        Self { series, movies }
    }

    async fn link(&self, files: &[LinkedFile]) -> Result<(), LibraryError> {
        for linked in files {
            self.update(linked.target, |file| *file = Some(linked.file)).await?;
        }
        Ok(())
    }

    async fn unlink(&self, deleted: MediaFileId, target: FileTarget) -> Result<(), LibraryError> {
        self.update(target, |file| {
            if *file == Some(deleted) {
                *file = None;
            }
        })
        .await
    }

    /// Items and episodes no longer in the library are skipped.
    async fn update(&self, target: FileTarget, change: impl Fn(&mut Option<MediaFileId>)) -> Result<(), LibraryError> {
        let change = &change;
        retry::on_conflict(|| async move {
            match target {
                FileTarget::Episodes { series, span } => {
                    let Some(mut series) = self.series.get(series).await? else { return Ok(()) };
                    for reference in span.refs() {
                        if let Some(episode) = series.episode_mut(reference) {
                            change(&mut episode.file);
                        }
                    }
                    self.series.save(&mut series, &[]).await?;
                },
                FileTarget::Movie(movie) => {
                    let Some(mut movie) = self.movies.get(movie).await? else { return Ok(()) };
                    change(&mut movie.file);
                    self.movies.save(&mut movie, &[]).await?;
                },
            }
            Ok(())
        })
        .await
    }
}

#[async_trait]
impl Subscriber for FileTracker {
    fn name(&self) -> &'static str {
        "library.files"
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        let event = &recorded.event;
        if let Some(FilesFound { files }) = event.get() {
            self.link(files).await?;
        } else if let Some(FilesImported { files, .. }) = event.get() {
            self.link(files).await?;
        } else if let Some(FileDeleted { file, target, .. }) = event.get() {
            self.unlink(*file, *target).await?;
        }
        Ok(())
    }
}
