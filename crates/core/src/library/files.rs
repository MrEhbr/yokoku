use std::sync::Arc;

use async_trait::async_trait;
use yokoku_domain::{
    FileTarget, MediaFileId,
    events::{Event, FilesFound, FilesImported},
};

use crate::{
    events::{HandlerError, Subscriber},
    library::{
        LibraryError,
        ports::{MediaFiles, MovieRepo, SeriesRepo},
        retry,
    },
};

/// Keeps the file id of each episode and movie in step with media's file events.
pub struct FileTracker {
    series: Arc<dyn SeriesRepo>,
    movies: Arc<dyn MovieRepo>,
    files: Arc<dyn MediaFiles>,
}

impl FileTracker {
    pub fn new(series: Arc<dyn SeriesRepo>, movies: Arc<dyn MovieRepo>, files: Arc<dyn MediaFiles>) -> Self {
        Self { series, movies, files }
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
                    self.series.save(&mut series).await?;
                },
                FileTarget::Movie(movie) => {
                    let Some(mut movie) = self.movies.get(movie).await? else { return Ok(()) };
                    change(&mut movie.file);
                    self.movies.save(&mut movie).await?;
                },
            }
            Ok(())
        })
        .await
    }
}

/// Links found and imported files where media holds them now, and unlinks deleted ones; files no
/// longer in the library are skipped.
#[async_trait]
impl Subscriber for FileTracker {
    fn name(&self) -> &'static str {
        "library.files"
    }

    async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        match event {
            Event::FilesFound(FilesFound { files }) | Event::FilesImported(FilesImported { files, .. }) => {
                for linked in files {
                    if let Some(target) = self.files.target(linked.file).await? {
                        self.update(target, |file| *file = Some(linked.file)).await?;
                    }
                }
            },
            Event::FileDeleted(deleted) => {
                self.update(deleted.target, |file| {
                    if *file == Some(deleted.file) {
                        *file = None;
                    }
                })
                .await?;
            },
            _ => {},
        }
        Ok(())
    }
}
