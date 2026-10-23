use std::collections::HashMap;

use async_trait::async_trait;
use tracing::info;
use yokoku_domain::{
    Confidence, FileTarget, ImportId, ItemId, MediaFileId,
    events::{Event, ImportNeedsReview},
};

use super::Scanner;
use crate::{
    events::{HandlerError, Subscriber},
    media::{Import, ImportRow, ImportStatus, MediaFile, Resolution, RowMatch, ports::Changes},
};

/// Scans the folder of each added item. On renumbered episodes, moves each file to the episodes now
/// holding it; a file whose episodes split leaves the library and goes to review. Files and series
/// no longer in the library are skipped.
#[async_trait]
impl Subscriber for Scanner {
    fn name(&self) -> &'static str {
        "media.scanner"
    }

    async fn handle(&self, event: &Event) -> Result<(), HandlerError> {
        let renumbered = match event {
            Event::SeriesAdded(added) => {
                self.scan_item(ItemId::Series(added.series)).await?;
                return Ok(());
            },
            Event::MovieAdded(added) => {
                self.scan_item(ItemId::Movie(added.movie)).await?;
                return Ok(());
            },
            Event::EpisodesRenumbered(renumbered) => renumbered,
            _ => return Ok(()),
        };
        let _lock = self.lock.acquire().await?;
        let Some(series) = self.catalog.series(renumbered.series).await? else { return Ok(()) };
        let files: HashMap<MediaFileId, MediaFile> =
            self.repo.files().await?.into_iter().map(|file| (file.id, file)).collect();

        let mut changes = Changes::default();
        let mut rows = Vec::new();
        for moved in &renumbered.files {
            let Some(file) = files.get(&moved.file) else { continue };
            match moved.span {
                Some(span) => {
                    changes.retargeted_files.push((file.id, FileTarget::Episodes { series: series.id, span }));
                },
                None => {
                    changes.removed_files.push(file.id);
                    rows.push(ImportRow {
                        path: file.path.clone(),
                        size: file.size,
                        matched: RowMatch::Series { series: series.id, season: None, episodes: None },
                        confidence: Confidence::Unknown,
                        skipped: true,
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
        info!(
            series = %series.id,
            retargeted = changes.retargeted_files.len(),
            to_review = changes.removed_files.len(),
            "files renumbered"
        );
        Ok(())
    }
}
