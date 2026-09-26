use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use tracing::warn;
use yokoku_domain::{ItemId, SubtitleTags};
use yokoku_events::{FilesFound, FilesImported, HandlerError, LinkedFile, Recorded, Subscriber};

use crate::{
    MediaError, MediaFile, MediaInfo, files,
    ports::{FileSystem, MediaProbe, MediaRepo, ProbeError},
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
        let mut details = Vec::new();
        for file in self.repo.files().await?.into_iter().filter(|file| file.target.item() == item) {
            let info = self.repo.media_info(file.id).await?;
            let subtitles = files::sidecar_subtitles(self.fs.as_ref(), &file.path).await?;
            let subtitle_files = subtitles.into_iter().map(|subtitle| subtitle.tags).collect();
            details.push(FileDetails { file, info, subtitle_files });
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
                Ok(info) => self.repo.save_media_info(file.file, &info).await?,
                Err(ProbeError::Missing) => {
                    warn!("ffprobe is not installed; file details stay unknown");
                    return Ok(());
                },
                Err(error) => warn!(%error, "could not read the streams of a new file"),
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Subscriber for Prober {
    fn name(&self) -> &'static str {
        "media.probe"
    }

    async fn handle(&self, recorded: &Recorded) -> Result<(), HandlerError> {
        let event = &recorded.event;
        if let Some(FilesFound { files }) = event.get() {
            self.probe_new(files).await?;
        } else if let Some(FilesImported { files, .. }) = event.get() {
            self.probe_new(files).await?;
        }
        Ok(())
    }
}
