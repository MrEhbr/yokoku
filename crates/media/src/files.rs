use std::{io, path::Path};

use yokoku_detect::{DownloadFile, Subtitle, classify};

use crate::ports::{FileSystem, FsError};

/// Subtitles beside `video` whose names start with the video's name.
pub(crate) async fn sidecar_subtitles(fs: &dyn FileSystem, video: &Path) -> Result<Vec<Subtitle>, FsError> {
    let (Some(folder), Some(stem)) = (video.parent(), video.file_stem()) else { return Ok(Vec::new()) };
    let prefix = format!("{}.", stem.to_string_lossy());
    let candidates: Vec<DownloadFile> = fs
        .files_in(folder)
        .await?
        .into_iter()
        .filter(|file| {
            file.path == video || file.path.file_name().is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        })
        .collect();

    let owner = classify(&candidates).videos.into_iter().find(|candidate| candidate.path == video);
    Ok(owner.map(|video| video.subtitles).unwrap_or_default())
}

/// Renames the file, or copies and then deletes it when the two paths are on different file
/// systems.
pub(crate) async fn move_file(fs: &dyn FileSystem, from: &Path, to: &Path) -> Result<(), FsError> {
    match fs.rename(from, to).await {
        Err(error) if error.source.kind() == io::ErrorKind::CrossesDevices => {
            fs.copy(from, to).await?;
            fs.remove_file(from).await
        },
        result => result,
    }
}
