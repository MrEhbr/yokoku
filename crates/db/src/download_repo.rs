use std::path::PathBuf;

use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_domain::{DownloadId, ItemId, MovieId, SeriesId};
use yokoku_downloads::{
    Download, DownloadState, DownloadStatus,
    ports::{DownloadRepo, StorageError},
};
use yokoku_events::Event;

use crate::{
    Database, DbError,
    codec::{Int, PathText, Text},
};

#[derive(sqlx::FromRow)]
struct DownloadRow {
    id: Text<DownloadId>,
    hash: String,
    name: String,
    series_id: Option<Text<SeriesId>>,
    movie_id: Option<Text<MovieId>>,
    state: Text<DownloadState>,
    size: u64,
    done: u64,
    download_rate: u64,
    eta: Option<u64>,
    download_dir: String,
    error: Option<String>,
    added_at: Text<Timestamp>,
    completed_at: Option<Text<Timestamp>>,
    imported_at: Option<Text<Timestamp>>,
    revision: u64,
}

#[async_trait]
impl DownloadRepo for Database {
    async fn get(&self, id: DownloadId) -> Result<Option<Download>, StorageError> {
        let row: Option<DownloadRow> = sqlx::query_as(
            "SELECT id, hash, name, series_id, movie_id, state, size, done, download_rate, eta, download_dir, error,
                    added_at, completed_at, imported_at, revision
             FROM downloads WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(row.map(Download::try_from).transpose()?)
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<Download>, StorageError> {
        let row: Option<DownloadRow> = sqlx::query_as(
            "SELECT id, hash, name, series_id, movie_id, state, size, done, download_rate, eta, download_dir, error,
                    added_at, completed_at, imported_at, revision
             FROM downloads WHERE hash = ?",
        )
        .bind(hash)
        .fetch_optional(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(row.map(Download::try_from).transpose()?)
    }

    async fn list(&self) -> Result<Vec<Download>, StorageError> {
        let rows: Vec<DownloadRow> = sqlx::query_as(
            "SELECT id, hash, name, series_id, movie_id, state, size, done, download_rate, eta, download_dir, error,
                    added_at, completed_at, imported_at, revision
             FROM downloads ORDER BY added_at DESC, id DESC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(rows.into_iter().map(Download::try_from).collect::<Result<_, _>>()?)
    }

    async fn save(&self, download: &mut Download, events: &[Event]) -> Result<(), StorageError> {
        Ok(self.save_download(download, events).await?)
    }
}

impl Database {
    async fn save_download(&self, download: &mut Download, events: &[Event]) -> Result<(), DbError> {
        let status = &download.status;
        let (series_id, movie_id) = match download.item {
            Some(ItemId::Series(id)) => (Some(id.to_string()), None),
            Some(ItemId::Movie(id)) => (None, Some(id.to_string())),
            None => (None, None),
        };

        let mut tx = self.begin().await?;
        if download.revision > 0 {
            let claimed = sqlx::query("UPDATE downloads SET revision = revision + 1 WHERE id = ? AND revision = ?")
                .bind(download.id.to_string())
                .bind(Int(download.revision))
                .execute(&mut *tx)
                .await?;
            if claimed.rows_affected() == 0 {
                return Err(DbError::Conflict);
            }
        }
        sqlx::query(
            "INSERT INTO downloads (id, hash, name, series_id, movie_id, state, size, done, download_rate, eta,
                                    download_dir, error, added_at, completed_at, imported_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name, series_id = excluded.series_id, movie_id = excluded.movie_id,
                 state = excluded.state, size = excluded.size, done = excluded.done,
                 download_rate = excluded.download_rate, eta = excluded.eta, download_dir = excluded.download_dir,
                 error = excluded.error, completed_at = excluded.completed_at, imported_at = excluded.imported_at",
        )
        .bind(download.id.to_string())
        .bind(&download.hash)
        .bind(&download.name)
        .bind(series_id)
        .bind(movie_id)
        .bind(status.state.as_str())
        .bind(Int(status.size))
        .bind(Int(status.done))
        .bind(Int(status.download_rate))
        .bind(status.eta.map(Int))
        .bind(PathText(&status.download_dir))
        .bind(&status.error)
        .bind(download.added_at.to_string())
        .bind(download.completed_at.map(|at| at.to_string()))
        .bind(download.imported_at.map(|at| at.to_string()))
        .execute(&mut *tx)
        .await
        .map_err(|error| match error.as_database_error() {
            Some(database) if database.is_unique_violation() => DbError::Conflict,
            _ => DbError::from(error),
        })?;
        self.commit(tx, events).await?;
        download.revision += 1;
        Ok(())
    }
}

impl TryFrom<DownloadRow> for Download {
    type Error = DbError;

    fn try_from(row: DownloadRow) -> Result<Self, Self::Error> {
        let item = match (row.series_id, row.movie_id) {
            (Some(id), None) => Some(ItemId::Series(id.0)),
            (None, Some(id)) => Some(ItemId::Movie(id.0)),
            (None, None) => None,
            (Some(_), Some(_)) => return Err(DbError::InvalidValue("download linked to a series and a movie".into())),
        };
        Ok(Download {
            id: row.id.0,
            hash: row.hash,
            name: row.name,
            item,
            status: DownloadStatus {
                state: row.state.0,
                size: row.size,
                done: row.done,
                download_rate: row.download_rate,
                eta: row.eta,
                download_dir: PathBuf::from(row.download_dir),
                error: row.error,
            },
            added_at: row.added_at.0,
            completed_at: row.completed_at.map(|at| at.0),
            imported_at: row.imported_at.map(|at| at.0),
            revision: row.revision,
        })
    }
}
