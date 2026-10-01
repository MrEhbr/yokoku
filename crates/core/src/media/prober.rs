use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use tracing::{debug, warn};
use yokoku_domain::{
    ItemId, SubtitleTags,
    events::{FilesFound, FilesImported, LinkedFile},
};

use crate::{
    events::{Handler, HandlerError},
    media::{
        MediaError, MediaFile, MediaInfo,
        detect::ListedFile,
        files,
        ports::{FileSystem, MediaProbe, MediaRepo, ProbeError},
    },
};

/// Reads and keeps the streams of library files (FR-8.6).
pub struct Prober {
    repo: Arc<dyn MediaRepo>,
    fs: Arc<dyn FileSystem>,
    probe: Arc<dyn MediaProbe>,
}

/// A library file with what is known about its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDetails {
    pub file: MediaFile,
    /// `None` until the file is probed.
    pub info: Option<MediaInfo>,
    /// Subtitle files beside the video.
    pub subtitle_files: Vec<SubtitleTags>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProbeReport {
    pub probed: usize,
    /// Paths that could not be probed, with the reason.
    pub failed: Vec<(PathBuf, String)>,
}

impl Prober {
    pub fn new(repo: Arc<dyn MediaRepo>, fs: Arc<dyn FileSystem>, probe: Arc<dyn MediaProbe>) -> Self {
        Self { repo, fs, probe }
    }

    /// The files of `item`, ordered by path.
    pub async fn details(&self, item: ItemId) -> Result<Vec<FileDetails>, MediaError> {
        let mut infos = self.repo.media_info_of(item).await?;
        let mut listings: HashMap<PathBuf, Vec<ListedFile>> = HashMap::new();
        let mut details = Vec::new();
        for file in self.repo.files_of(item).await? {
            let folder = file.path.parent().map(Path::to_owned).unwrap_or_default();
            if !listings.contains_key(&folder) {
                let listed = files::files_beside(self.fs.as_ref(), &folder).await?;
                listings.insert(folder.clone(), listed);
            }
            let subtitles = files::subtitles_of(&listings[&folder], &file.path);
            let subtitle_files = subtitles.into_iter().map(|subtitle| subtitle.tags).collect();
            details.push(FileDetails { info: infos.remove(&file.id), file, subtitle_files });
        }
        Ok(details)
    }

    /// Probes every file never probed; fails without a probe installed.
    pub async fn probe_missing(&self) -> Result<ProbeReport, MediaError> {
        let mut report = ProbeReport::default();
        for file in self.repo.files_without_media_info().await? {
            match self.probe.probe(&file.path).await {
                Ok(info) => {
                    self.repo.save_media_info(file.id, &info).await?;
                    report.probed += 1;
                },
                Err(ProbeError::Missing) => return Err(ProbeError::Missing.into()),
                Err(error) => report.failed.push((file.path, error.to_string())),
            }
        }
        Ok(report)
    }

    /// A file that cannot be probed, or a missing probe, is only logged; `probe_missing` tries it again.
    async fn probe_new(&self, files: &[LinkedFile]) -> Result<(), MediaError> {
        for file in files {
            match self.probe.probe(&file.path).await {
                Ok(info) => {
                    self.repo.save_media_info(file.file, &info).await?;
                    debug!(path = %file.path.display(), "probed");
                },
                Err(ProbeError::Missing) => {
                    warn!("ffprobe is not installed; file details stay unknown");
                    return Ok(());
                },
                Err(error) => warn!(%error, path = %file.path.display(), "could not read the streams of a new file"),
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Handler<FilesFound> for Prober {
    async fn handle(&self, event: &FilesFound) -> Result<(), HandlerError> {
        Ok(self.probe_new(&event.files).await?)
    }
}

#[async_trait]
impl Handler<FilesImported> for Prober {
    async fn handle(&self, event: &FilesImported) -> Result<(), HandlerError> {
        Ok(self.probe_new(&event.files).await?)
    }
}
