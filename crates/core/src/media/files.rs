use std::{io, path::Path};

use crate::media::{
    detect::{Classified, ListedFile, Subtitle},
    ports::{FileSystem, FsError},
};

/// Subtitles beside `video` whose names start with the video's name; none when its folder is gone.
pub(crate) async fn sidecar_subtitles(fs: &dyn FileSystem, video: &Path) -> Result<Vec<Subtitle>, FsError> {
    let Some(folder) = video.parent() else { return Ok(Vec::new()) };
    Ok(subtitles_of(&files_beside(fs, folder).await?, video))
}

/// The files directly in `folder`; none when it is gone.
pub(crate) async fn files_beside(fs: &dyn FileSystem, folder: &Path) -> Result<Vec<ListedFile>, FsError> {
    match fs.files_in(folder).await {
        Err(error) if error.source.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        listed => listed,
    }
}

/// The subtitles among `listed`, the files of `video`'s folder, whose names start with the video's name.
pub(crate) fn subtitles_of(listed: &[ListedFile], video: &Path) -> Vec<Subtitle> {
    let Some(stem) = video.file_stem() else { return Vec::new() };
    let prefix = format!("{}.", stem.to_string_lossy());
    let candidates: Vec<ListedFile> = listed
        .iter()
        .filter(|file| {
            file.path == video || file.path.file_name().is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        })
        .cloned()
        .collect();

    let owner = Classified::from_files(&candidates).videos.into_iter().find(|candidate| candidate.path == video);
    owner.map(|video| video.subtitles).unwrap_or_default()
}
