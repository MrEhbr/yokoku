use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use jiff::Timestamp;
use yokoku_core::{
    library::ports::MediaFiles,
    media::{
        Episodes, Import, ImportRow, ImportStatus, MediaFile, MediaInfo, Resolution, RootFolder, RootKind, RowMatch,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{
    Confidence, DownloadId, EpisodeSpan, FileTarget, ImportId, ItemId, MediaFileId, MovieId, SeriesId, StorageError,
};

use crate::db::{
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
    resolution: Text<Resolution>,
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

    async fn files_of(&self, item: ItemId) -> Result<Vec<MediaFile>, StorageError> {
        let rows: Vec<MediaFileRow> = sqlx::query_as(
            "SELECT id, path, size, series_id, season, first_episode, last_episode, movie_id, added_at
             FROM media_files WHERE series_id = ? OR movie_id = ? ORDER BY path",
        )
        .bind(item.series().map(Text))
        .bind(item.movie().map(Text))
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

    async fn claimed_paths(&self) -> Result<Vec<PathBuf>, StorageError> {
        let paths: Vec<String> = sqlx::query_scalar(
            "SELECT import_rows.path FROM import_rows JOIN imports ON imports.id = import_rows.import_id
             WHERE imports.status != 'done' OR import_rows.skipped",
        )
        .fetch_all(self.pool())
        .await
        .map_err(DbError::from)?;
        Ok(paths.into_iter().map(PathBuf::from).collect())
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

    async fn media_info_of(&self, item: ItemId) -> Result<HashMap<MediaFileId, MediaInfo>, StorageError> {
        Ok(self.load_media_info_of(item).await?)
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

    async fn save(&self, changes: &Changes) -> Result<(), StorageError> {
        Ok(self.save_changes(changes).await?)
    }
}

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

impl Database {
    async fn load_import(&self, record: ImportRecord) -> Result<Import, DbError> {
        let rows: Vec<ImportRowRecord> = sqlx::query_as(
            "SELECT path, size, series_id, season, first_episode, last_episode, movie_id, confidence, skipped,
                    resolution
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

    async fn save_changes(&self, changes: &Changes) -> Result<(), DbError> {
        let mut tx = self.pool().begin().await?;
        for id in &changes.removed_files {
            sqlx::query("DELETE FROM media_files WHERE id = ?").bind(id.to_string()).execute(&mut *tx).await?;
        }
        for file in &changes.added_files {
            let target = TargetColumns::from(Some(file.target));
            sqlx::query(
                "INSERT INTO media_files (id, path, size, added_at, series_id, season, first_episode, last_episode,
                                          movie_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(file.id.to_string())
            .bind(PathText(&file.path))
            .bind(Int(file.size))
            .bind(file.added_at.to_string())
            .bind(target.series_id)
            .bind(target.season)
            .bind(target.first_episode)
            .bind(target.last_episode)
            .bind(target.movie_id)
            .execute(&mut *tx)
            .await?;
        }
        for (id, path) in &changes.renamed_files {
            sqlx::query("UPDATE media_files SET path = ? WHERE id = ?")
                .bind(PathText(path))
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        for (id, target) in &changes.retargeted_files {
            let target = TargetColumns::from(Some(*target));
            sqlx::query(
                "UPDATE media_files SET series_id = ?, season = ?, first_episode = ?, last_episode = ?, movie_id = ?
                 WHERE id = ?",
            )
            .bind(target.series_id)
            .bind(target.season)
            .bind(target.first_episode)
            .bind(target.last_episode)
            .bind(target.movie_id)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        }
        for import in &changes.imports {
            let id = import.id.to_string();
            sqlx::query(
                "INSERT INTO imports (id, source, download_id, status, error, created_at) VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT (id) DO UPDATE SET
                     source = excluded.source, status = excluded.status, error = excluded.error",
            )
            .bind(&id)
            .bind(PathText(&import.source))
            .bind(import.download.map(|download| download.to_string()))
            .bind(import.status.as_str())
            .bind(&import.error)
            .bind(import.created_at.to_string())
            .execute(&mut *tx)
            .await?;

            sqlx::query("DELETE FROM import_rows WHERE import_id = ?").bind(&id).execute(&mut *tx).await?;
            for (position, row) in (0_i64..).zip(&import.rows) {
                let target = TargetColumns::from(row.matched);
                sqlx::query(
                    "INSERT INTO import_rows (import_id, position, path, size, confidence, skipped, resolution,
                                              series_id, season, first_episode, last_episode, movie_id)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(position)
                .bind(PathText(&row.path))
                .bind(Int(row.size))
                .bind(row.confidence.as_str())
                .bind(row.skipped)
                .bind(row.resolution.as_str())
                .bind(target.series_id)
                .bind(target.season)
                .bind(target.first_episode)
                .bind(target.last_episode)
                .bind(target.movie_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        Ok(tx.commit().await?)
    }
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
            matched: row.target.into_row_match()?,
            confidence: row.confidence.0,
            skipped: row.skipped,
            resolution: row.resolution.0,
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

impl From<RowMatch> for TargetColumns {
    fn from(matched: RowMatch) -> Self {
        let empty = Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: None };
        match matched {
            RowMatch::None => empty,
            RowMatch::Series { series, season, episodes } => Self {
                series_id: Some(Text(series)),
                season,
                first_episode: episodes.map(|episodes| episodes.first),
                last_episode: episodes.map(|episodes| episodes.last),
                ..empty
            },
            RowMatch::Movie(movie) => Self { movie_id: Some(Text(movie)), ..empty },
        }
    }
}

impl TargetColumns {
    fn into_row_match(self) -> Result<RowMatch, DbError> {
        match self {
            Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id } => {
                Ok(movie_id.map_or(RowMatch::None, |movie| RowMatch::Movie(movie.0)))
            },
            Self { series_id: Some(series), season, first_episode, last_episode, movie_id: None } => {
                let episodes = match (first_episode, last_episode) {
                    (None, None) => None,
                    (Some(first), Some(last)) if first <= last => Some(Episodes { first, last }),
                    _ => return Err(DbError::InvalidValue(format!("episodes {first_episode:?}-{last_episode:?}"))),
                };
                Ok(RowMatch::Series { series: series.0, season, episodes })
            },
            _ => Err(DbError::InvalidValue("import row matched to a series and a movie".into())),
        }
    }

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
