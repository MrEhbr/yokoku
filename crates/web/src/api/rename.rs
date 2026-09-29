//! Renaming an item's files to the naming patterns, with a preview first (FR-5.7).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ItemId, MediaFileId};

#[cfg(feature = "server")]
use crate::api::{Dep, Renamer};

/// The files whose names would change, by season and episode, and those left as they are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenamePlan {
    pub renames: Vec<RenameRow>,
    pub skipped: Vec<SkippedFile>,
}

/// Paths are relative to the item's folder, which a rename keeps, or else to the root folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenameRow {
    pub file: MediaFileId,
    /// `None` for a movie.
    pub season: Option<u16>,
    pub from: String,
    pub to: String,
    /// Subtitle files renamed with it.
    pub subtitles: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenameResult {
    pub renamed: usize,
    pub failed: Vec<SkippedFile>,
}

#[post("/api/rename/preview", renamer: Dep<Renamer>)]
pub async fn rename_preview(item: ItemId) -> Result<RenamePlan, ServerFnError> {
    server::preview(&renamer, item).await
}

/// Renames the item's `files` as the plan says; files the plan no longer holds are left alone.
#[post("/api/rename", renamer: Dep<Renamer>)]
pub async fn rename_files(item: ItemId, files: Vec<MediaFileId>) -> Result<RenameResult, ServerFnError> {
    server::apply(&renamer, item, &files).await
}

#[cfg(feature = "server")]
mod server {
    use std::path::Path;

    use dioxus::{logger::tracing::error, prelude::*};
    use yokoku_domain::{FileTarget, ItemId, MediaFileId};
    use yokoku_media::{MediaError, Rename, RenameScope, Skipped};

    use super::{RenamePlan, RenameResult, RenameRow, Renamer, SkippedFile};

    pub(super) async fn preview(renamer: &Renamer, item: ItemId) -> Result<RenamePlan, ServerFnError> {
        let plan = renamer.preview(scope(item)).await.map_err(|error| failure(error, "previewing renames"))?;
        let mut renames: Vec<RenameRow> = plan.renames.iter().map(RenameRow::from).collect();
        renames.sort_by(|a, b| (a.season, &a.to).cmp(&(b.season, &b.to)));
        Ok(RenamePlan { renames, skipped: plan.skipped.into_iter().map(SkippedFile::from).collect() })
    }

    pub(super) async fn apply(
        renamer: &Renamer,
        item: ItemId,
        files: &[MediaFileId],
    ) -> Result<RenameResult, ServerFnError> {
        let report = renamer.apply(scope(item), Some(files)).await.map_err(|error| failure(error, "renaming"))?;
        Ok(RenameResult {
            renamed: report.renamed.len(),
            failed: report
                .failed
                .into_iter()
                .map(|failure| SkippedFile { path: failure.path.display().to_string(), reason: failure.error })
                .collect(),
        })
    }

    fn scope(item: ItemId) -> RenameScope {
        match item {
            ItemId::Series(id) => RenameScope::Series(id),
            ItemId::Movie(id) => RenameScope::Movie(id),
        }
    }

    fn failure(error: MediaError, doing: &str) -> ServerFnError {
        error!(%error, "{doing} failed");
        ServerFnError::new("Something went wrong; the server log has the cause")
    }

    impl From<&Rename> for RenameRow {
        fn from(rename: &Rename) -> Self {
            let folder = rename.video.to.strip_prefix(&rename.root).ok().and_then(|path| path.components().next());
            let base = folder.map_or_else(|| rename.root.clone(), |folder| rename.root.join(folder));
            let relative = |path: &Path| {
                path.strip_prefix(&base)
                    .or_else(|_| path.strip_prefix(&rename.root))
                    .unwrap_or(path)
                    .display()
                    .to_string()
            };
            Self {
                file: rename.file,
                season: match rename.target {
                    FileTarget::Episodes { span, .. } => Some(span.season()),
                    FileTarget::Movie(_) => None,
                },
                from: relative(&rename.video.from),
                to: relative(&rename.video.to),
                subtitles: rename.subtitles.iter().filter(|subtitle| subtitle.from != subtitle.to).count(),
            }
        }
    }

    impl From<Skipped> for SkippedFile {
        fn from(skipped: Skipped) -> Self {
            Self { path: skipped.path.display().to_string(), reason: skipped.reason.to_string() }
        }
    }
}
