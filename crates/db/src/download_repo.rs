use std::path::PathBuf;

use async_trait::async_trait;
use yokoku_domain::{DownloadId, ItemId, MovieId, SeriesId};
use yokoku_downloads::{
    Download, DownloadState, DownloadStatus,
    ports::{DownloadRepo, StorageError},
};
use yokoku_events::Event;

use crate::{Database, DbError, codec};

#[derive(sqlx::FromRow)]
struct DownloadRow {
    id: String,
    hash: String,
    name: String,
    series_id: Option<String>,
    movie_id: Option<String>,
    state: String,
    size: i64,
    done: i64,
    download_rate: i64,
    eta: Option<i64>,
    download_dir: String,
    error: Option<String>,
    added_at: String,
    completed_at: Option<String>,
    imported_at: Option<String>,
    revision: i64,
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
        Ok(row.map(download).transpose()?)
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
        Ok(row.map(download).transpose()?)
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
        Ok(rows.into_iter().map(download).collect::<Result<_, _>>()?)
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
        let download_dir = status
            .download_dir
            .to_str()
            .ok_or_else(|| DbError::InvalidValue(format!("path {} is not UTF-8", status.download_dir.display())))?;

        let mut tx = self.begin().await?;
        if download.revision > 0 {
            let claimed = sqlx::query("UPDATE downloads SET revision = revision + 1 WHERE id = ? AND revision = ?")
                .bind(download.id.to_string())
                .bind(codec::revision_to_i64(download.revision)?)
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
        .bind(state_to_str(status.state))
        .bind(to_i64(status.size)?)
        .bind(to_i64(status.done)?)
        .bind(to_i64(status.download_rate)?)
        .bind(status.eta.map(to_i64).transpose()?)
        .bind(download_dir)
        .bind(&status.error)
        .bind(download.added_at.to_string())
        .bind(download.completed_at.map(|at| at.to_string()))
        .bind(download.imported_at.map(|at| at.to_string()))
        .execute(&mut *tx)
        .await?;
        self.commit(tx, events).await?;
        download.revision += 1;
        Ok(())
    }
}

fn download(row: DownloadRow) -> Result<Download, DbError> {
    let item = match (row.series_id, row.movie_id) {
        (Some(id), None) => Some(ItemId::Series(SeriesId(codec::uuid(&id)?))),
        (None, Some(id)) => Some(ItemId::Movie(MovieId(codec::uuid(&id)?))),
        (None, None) => None,
        (Some(_), Some(_)) => return Err(DbError::InvalidValue("download linked to a series and a movie".into())),
    };
    Ok(Download {
        id: DownloadId(codec::uuid(&row.id)?),
        hash: row.hash,
        name: row.name,
        item,
        status: DownloadStatus {
            state: state_from_str(&row.state)?,
            size: from_i64(row.size)?,
            done: from_i64(row.done)?,
            download_rate: from_i64(row.download_rate)?,
            eta: row.eta.map(from_i64).transpose()?,
            download_dir: PathBuf::from(row.download_dir),
            error: row.error,
        },
        added_at: codec::timestamp(&row.added_at)?,
        completed_at: row.completed_at.as_deref().map(codec::timestamp).transpose()?,
        imported_at: row.imported_at.as_deref().map(codec::timestamp).transpose()?,
        revision: codec::revision(row.revision)?,
    })
}

fn state_to_str(state: DownloadState) -> &'static str {
    match state {
        DownloadState::Queued => "queued",
        DownloadState::Checking => "checking",
        DownloadState::Downloading => "downloading",
        DownloadState::Seeding => "seeding",
        DownloadState::Stopped => "stopped",
        DownloadState::Removed => "removed",
    }
}

fn state_from_str(value: &str) -> Result<DownloadState, DbError> {
    Ok(match value {
        "queued" => DownloadState::Queued,
        "checking" => DownloadState::Checking,
        "downloading" => DownloadState::Downloading,
        "seeding" => DownloadState::Seeding,
        "stopped" => DownloadState::Stopped,
        "removed" => DownloadState::Removed,
        other => return Err(DbError::InvalidValue(format!("download state {other:?}"))),
    })
}

fn to_i64(value: u64) -> Result<i64, DbError> {
    i64::try_from(value).map_err(|_| DbError::InvalidValue(format!("{value} out of range")))
}

fn from_i64(value: i64) -> Result<u64, DbError> {
    u64::try_from(value).map_err(|_| DbError::InvalidValue(format!("{value} out of range")))
}
