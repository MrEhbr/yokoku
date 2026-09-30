use async_trait::async_trait;
use yokoku_core::{library::ports::MediaFiles, media::MediaFile};
use yokoku_domain::{FileTarget, MediaFileId, StorageError};

use crate::db::{Database, DbError, media_repo::MediaFileRow};

#[async_trait]
impl MediaFiles for Database {
    async fn target(&self, file: MediaFileId) -> Result<Option<FileTarget>, StorageError> {
        let row: Option<MediaFileRow> = sqlx::query_as(
            "SELECT id, path, size, series_id, season, first_episode, last_episode, movie_id, added_at
             FROM media_files WHERE id = ?",
        )
        .bind(file.to_string())
        .fetch_optional(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(row.map(MediaFile::try_from).transpose()?.map(|file| file.target))
    }
}
