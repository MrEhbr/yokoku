use std::collections::HashMap;

use async_trait::async_trait;
use tracing::{info, instrument};
use yokoku_domain::{Confidence, FileTarget, ImportId, MediaFileId};
use yokoku_events::{EpisodesRenumbered, Event, Handler, HandlerError, ImportNeedsReview};

use super::Scanner;
use crate::{Import, ImportRow, ImportStatus, MediaError, MediaFile, Resolution, ports::Changes};

impl Scanner {
    /// Moves each file to the episodes now holding it; a file whose episodes split leaves the
    /// library and goes to review. Files and series no longer in the library are skipped.
    #[instrument(skip_all, fields(series = %event.series))]
    async fn retarget(&self, event: &EpisodesRenumbered) -> Result<(), MediaError> {
        let _lock = self.lock.acquire().await?;
        let Some(series) = self.catalog.series(event.series).await? else { return Ok(()) };
        let files: HashMap<MediaFileId, MediaFile> =
            self.repo.files().await?.into_iter().map(|file| (file.id, file)).collect();

        let mut changes = Changes::default();
        let mut rows = Vec::new();
        for renumbered in &event.files {
            let Some(file) = files.get(&renumbered.file) else { continue };
            match renumbered.span {
                Some(span) => {
                    changes.retargeted_files.push((file.id, FileTarget::Episodes { series: series.id, span }));
                },
                None => {
                    changes.removed_files.push(file.id);
                    rows.push(ImportRow {
                        path: file.path.clone(),
                        size: file.size,
                        target: None,
                        confidence: Confidence::Unknown,
                        skipped: false,
                        resolution: Resolution::Unresolved,
                    });
                },
            }
        }
        let mut events: Vec<Event> = Vec::new();
        if !rows.is_empty() {
            rows.sort_by(|a, b| a.path.cmp(&b.path));
            let import = Import {
                id: ImportId::generate(),
                source: series.folder.path(),
                download: None,
                status: ImportStatus::NeedsReview,
                error: None,
                rows,
                created_at: self.clock.now().timestamp(),
            };
            events.push(ImportNeedsReview { import: import.id, source: import.source.clone() }.into());
            changes.imports.push(import);
        }
        self.repo.save(&changes).await?;
        self.events.publish_all(events).await;
        info!(retargeted = changes.retargeted_files.len(), to_review = changes.removed_files.len(), "files renumbered");
        Ok(())
    }
}

#[async_trait]
impl Handler<EpisodesRenumbered> for Scanner {
    async fn handle(&self, event: &EpisodesRenumbered) -> Result<(), HandlerError> {
        self.retarget(event).await?;
        Ok(())
    }
}
