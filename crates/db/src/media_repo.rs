use std::path::{Path, PathBuf};

use async_trait::async_trait;
use jiff::Timestamp;
use sqlx::{Sqlite, Transaction};
use yokoku_domain::{
    Confidence, DownloadId, EpisodeSpan, FileTarget, ImportId, MediaFileId, Movie, MovieId, Series, SeriesId,
};
use yokoku_events::Event;
use yokoku_media::{
    Import, ImportRow, ImportStatus, MediaFile, MediaInfo, RootFolder, RootKind,
    ports::{Catalog, Changes, MediaRepo, StorageError},
};

use crate::{
    Database, DbError,
    codec::{Int, PathText, Text},
};

#[derive(sqlx::FromRow)]
struct RootFolderRow {
    path: String,
    kind: Text<RootKind>,
}

#[derive(sqlx::FromRow)]
struct MediaFileRow {
    id: Text<MediaFileId>,
    path: String,
    size: u64,
    #[sqlx(flatten)]
    target: TargetColumns,
    added_at: Text<Timestamp>,
}

#[derive(sqlx::FromRow)]
struct ImportRecord {
    id: Text<ImportId>,
    source: String,
    download_id: Option<Text<DownloadId>>,
    status: Text<ImportStatus>,
    error: Option<String>,
    created_at: Text<Timestamp>,
}

#[derive(sqlx::FromRow)]
struct ImportRowRecord {
    path: String,
    size: u64,
    #[sqlx(flatten)]
    target: TargetColumns,
    confidence: Text<Confidence>,
    skipped: bool,
    replace_file: bool,
}

#[derive(sqlx::FromRow)]
struct TargetColumns {
    series_id: Option<Text<SeriesId>>,
    season: Option<u16>,
    first_episode: Option<u16>,
    last_episode: Option<u16>,
    movie_id: Option<Text<MovieId>>,
}

#[async_trait]
impl MediaRepo for Database {
    async fn root_folders(&self) -> Result<Vec<RootFolder>, StorageError> {
        let rows: Vec<RootFolderRow> = sqlx::query_as("SELECT path, kind FROM root_folders ORDER BY path")
            .fetch_all(self.pool())
            .await
            .map_err(DbError::from)?;

        Ok(rows.into_iter().map(RootFolder::from).collect())
    }

    async fn add_root_folder(&self, root: &RootFolder) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO root_folders (path, kind) VALUES (?, ?)")
            .bind(PathText(&root.path))
            .bind(root.kind.as_str())
            .execute(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(())
    }

    async fn remove_root_folder(&self, path: &Path) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM root_folders WHERE path = ?")
            .bind(PathText(path))
            .execute(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(result.rows_affected() > 0)
    }

    async fn files(&self) -> Result<Vec<MediaFile>, StorageError> {
        let rows: Vec<MediaFileRow> = sqlx::query_as(
            "SELECT id, path, size, series_id, season, first_episode, last_episode, movie_id, added_at
             FROM media_files ORDER BY path",
        )
        .fetch_all(self.pool())
        .await
        .map_err(DbError::from)?;

        Ok(rows.into_iter().map(MediaFile::try_from).collect::<Result<_, _>>()?)
    }

    async fn import(&self, id: ImportId) -> Result<Option<Import>, StorageError> {
        let record: Option<ImportRecord> =
            sqlx::query_as("SELECT id, source, download_id, status, error, created_at FROM imports WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(self.pool())
                .await
                .map_err(DbError::from)?;

        match record {
            Some(record) => Ok(Some(self.load_import(record).await?)),
            None => Ok(None),
        }
    }

    async fn imports(&self, status: ImportStatus) -> Result<Vec<Import>, StorageError> {
        let records: Vec<ImportRecord> = sqlx::query_as(
            "SELECT id, source, download_id, status, error, created_at FROM imports WHERE status = ?
             ORDER BY created_at, id",
        )
        .bind(status.as_str())
        .fetch_all(self.pool())
        .await
        .map_err(DbError::from)?;

        let mut imports = Vec::with_capacity(records.len());
        for record in records {
            imports.push(self.load_import(record).await?);
        }
        Ok(imports)
    }

    async fn import_for_download(&self, download: DownloadId) -> Result<Option<Import>, StorageError> {
        let record: Option<ImportRecord> = sqlx::query_as(
            "SELECT id, source, download_id, status, error, created_at FROM imports WHERE download_id = ?",
        )
        .bind(download.to_string())
        .fetch_optional(self.pool())
        .await
        .map_err(DbError::from)?;

        match record {
            Some(record) => Ok(Some(self.load_import(record).await?)),
            None => Ok(None),
        }
    }

    async fn claim_next_approved(&self) -> Result<Option<Import>, StorageError> {
        let record: Option<ImportRecord> = sqlx::query_as(
            "UPDATE imports SET status = 'importing'
             WHERE id = (SELECT id FROM imports WHERE status = 'approved' ORDER BY created_at, id LIMIT 1)
             RETURNING id, source, download_id, status, error, created_at",
        )
        .fetch_optional(self.pool())
        .await
        .map_err(DbError::from)?;

        match record {
            Some(record) => Ok(Some(self.load_import(record).await?)),
            None => Ok(None),
        }
    }

    async fn reset_importing(&self) -> Result<u64, StorageError> {
        let result = sqlx::query("UPDATE imports SET status = 'approved' WHERE status = 'importing'")
            .execute(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(result.rows_affected())
    }

    async fn media_info(&self, file: MediaFileId) -> Result<Option<MediaInfo>, StorageError> {
        Ok(self.load_media_info(file).await?)
    }

    async fn save_media_info(&self, file: MediaFileId, info: &MediaInfo) -> Result<(), StorageError> {
        Ok(self.store_media_info(file, info).await?)
    }

    async fn files_without_media_info(&self) -> Result<Vec<MediaFile>, StorageError> {
        let rows: Vec<MediaFileRow> = sqlx::query_as(
            "SELECT id, path, size, series_id, season, first_episode, last_episode, movie_id, added_at
             FROM media_files WHERE id NOT IN (SELECT file_id FROM media_info) ORDER BY path",
        )
        .fetch_all(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(rows.into_iter().map(MediaFile::try_from).collect::<Result<_, _>>()?)
    }

    async fn save(&self, changes: &Changes, events: &[Event]) -> Result<(), StorageError> {
        let mut tx = self.begin().await?;
        for id in &changes.removed_files {
            sqlx::query("DELETE FROM media_files WHERE id = ?")
                .bind(id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(DbError::from)?;
        }
        for file in &changes.added_files {
            insert_file(&mut tx, file).await?;
        }
        for (id, path) in &changes.renamed_files {
            sqlx::query("UPDATE media_files SET path = ? WHERE id = ?")
                .bind(PathText(path))
                .bind(id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(DbError::from)?;
        }
        for import in &changes.imports {
            save_import(&mut tx, import).await?;
        }
        Ok(self.commit(tx, events).await?)
    }
}

#[async_trait]
impl Catalog for Database {
    async fn all_series(&self) -> Result<Vec<Series>, StorageError> {
        let mut series = Vec::new();
        for id in self.series_ids().await? {
            series.extend(self.load_series(id).await?);
        }
        Ok(series)
    }

    async fn all_movies(&self) -> Result<Vec<Movie>, StorageError> {
        let mut movies = Vec::new();
        for id in self.movie_ids().await? {
            movies.extend(self.load_movie(id).await?);
        }
        Ok(movies)
    }

    async fn series(&self, id: SeriesId) -> Result<Option<Series>, StorageError> {
        Ok(self.load_series(id).await?)
    }

    async fn movie(&self, id: MovieId) -> Result<Option<Movie>, StorageError> {
        Ok(self.load_movie(id).await?)
    }
}

impl Database {
    async fn load_import(&self, record: ImportRecord) -> Result<Import, DbError> {
        let rows: Vec<ImportRowRecord> = sqlx::query_as(
            "SELECT path, size, series_id, season, first_episode, last_episode, movie_id, confidence, skipped,
                    replace_file
             FROM import_rows WHERE import_id = ? ORDER BY position",
        )
        .bind(&record.id)
        .fetch_all(self.pool())
        .await?;

        Ok(Import {
            id: record.id.0,
            source: PathBuf::from(record.source),
            download: record.download_id.map(|download| download.0),
            status: record.status.0,
            error: record.error,
            rows: rows.into_iter().map(ImportRow::try_from).collect::<Result<_, _>>()?,
            created_at: record.created_at.0,
        })
    }
}

async fn insert_file(tx: &mut Transaction<'static, Sqlite>, file: &MediaFile) -> Result<(), DbError> {
    let target = TargetColumns::from(Some(file.target));
    sqlx::query(
        "INSERT INTO media_files (id, path, size, series_id, season, first_episode, last_episode, movie_id, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(file.id.to_string())
    .bind(PathText(&file.path))
    .bind(Int(file.size))
    .bind(target.series_id)
    .bind(target.season)
    .bind(target.first_episode)
    .bind(target.last_episode)
    .bind(target.movie_id)
    .bind(file.added_at.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Replaces the import and all its rows.
async fn save_import(tx: &mut Transaction<'static, Sqlite>, import: &Import) -> Result<(), DbError> {
    let id = import.id.to_string();
    sqlx::query(
        "INSERT INTO imports (id, source, download_id, status, error, created_at) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET source = excluded.source, status = excluded.status, error = excluded.error",
    )
    .bind(&id)
    .bind(PathText(&import.source))
    .bind(import.download.map(|download| download.to_string()))
    .bind(import.status.as_str())
    .bind(&import.error)
    .bind(import.created_at.to_string())
    .execute(&mut **tx)
    .await?;

    sqlx::query("DELETE FROM import_rows WHERE import_id = ?").bind(&id).execute(&mut **tx).await?;
    for (position, row) in (0_i64..).zip(&import.rows) {
        let target = TargetColumns::from(row.target);
        sqlx::query(
            "INSERT INTO import_rows (import_id, position, path, size, series_id, season, first_episode,
                                      last_episode, movie_id, confidence, skipped, replace_file)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(position)
        .bind(PathText(&row.path))
        .bind(Int(row.size))
        .bind(target.series_id)
        .bind(target.season)
        .bind(target.first_episode)
        .bind(target.last_episode)
        .bind(target.movie_id)
        .bind(row.confidence.as_str())
        .bind(row.skipped)
        .bind(row.replace)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

impl From<RootFolderRow> for RootFolder {
    fn from(row: RootFolderRow) -> Self {
        RootFolder { kind: row.kind.0, path: PathBuf::from(row.path) }
    }
}

impl TryFrom<MediaFileRow> for MediaFile {
    type Error = DbError;

    fn try_from(row: MediaFileRow) -> Result<Self, Self::Error> {
        Ok(MediaFile {
            id: row.id.0,
            path: PathBuf::from(row.path),
            size: row.size,
            target: row
                .target
                .into_target()?
                .ok_or_else(|| DbError::InvalidValue("media file without target".into()))?,
            added_at: row.added_at.0,
        })
    }
}

impl TryFrom<ImportRowRecord> for ImportRow {
    type Error = DbError;

    fn try_from(row: ImportRowRecord) -> Result<Self, Self::Error> {
        Ok(ImportRow {
            path: PathBuf::from(row.path),
            size: row.size,
            target: row.target.into_target()?,
            confidence: row.confidence.0,
            skipped: row.skipped,
            replace: row.replace_file,
        })
    }
}

impl From<Option<FileTarget>> for TargetColumns {
    fn from(target: Option<FileTarget>) -> Self {
        let empty = Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: None };
        match target {
            None => empty,
            Some(FileTarget::Episodes { series, span }) => Self {
                series_id: Some(Text(series)),
                season: Some(span.season()),
                first_episode: Some(span.first()),
                last_episode: Some(span.last()),
                ..empty
            },
            Some(FileTarget::Movie(movie)) => Self { movie_id: Some(Text(movie)), ..empty },
        }
    }
}

impl TargetColumns {
    fn into_target(self) -> Result<Option<FileTarget>, DbError> {
        match self {
            Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: None } => Ok(None),
            Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: Some(movie) } => {
                Ok(Some(FileTarget::Movie(movie.0)))
            },
            Self {
                series_id: Some(series),
                season: Some(season),
                first_episode: Some(first),
                last_episode: Some(last),
                movie_id: None,
            } => {
                let span = EpisodeSpan::new(season, first, last)
                    .ok_or_else(|| DbError::InvalidValue(format!("episodes {first}-{last}")))?;
                Ok(Some(FileTarget::Episodes { series: series.0, span }))
            },
            _ => Err(DbError::InvalidValue("incomplete file target".into())),
        }
    }
}
