use std::path::{Path, PathBuf};

use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};
use yokoku_domain::{Confidence, EpisodeSpan, FileTarget, ImportId, MediaFileId, Movie, MovieId, Series, SeriesId};
use yokoku_events::Event;
use yokoku_media::{
    Import, ImportRow, ImportStatus, MediaFile, RootFolder, RootKind,
    ports::{Catalog, Changes, MediaRepo, StorageError},
};

use crate::{Database, DbError, codec};

#[derive(sqlx::FromRow)]
struct RootFolderRow {
    path: String,
    kind: String,
}

#[derive(sqlx::FromRow)]
struct MediaFileRow {
    id: String,
    path: String,
    size: i64,
    #[sqlx(flatten)]
    target: TargetColumns,
    added_at: String,
}

#[derive(sqlx::FromRow)]
struct ImportRecord {
    id: String,
    source: String,
    status: String,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct ImportRowRecord {
    path: String,
    size: i64,
    #[sqlx(flatten)]
    target: TargetColumns,
    confidence: String,
    skipped: bool,
}

#[derive(sqlx::FromRow)]
struct TargetColumns {
    series_id: Option<String>,
    season: Option<u16>,
    first_episode: Option<u16>,
    last_episode: Option<u16>,
    movie_id: Option<String>,
}

#[async_trait]
impl MediaRepo for Database {
    async fn root_folders(&self) -> Result<Vec<RootFolder>, StorageError> {
        let rows: Vec<RootFolderRow> = sqlx::query_as("SELECT path, kind FROM root_folders ORDER BY path")
            .fetch_all(self.pool())
            .await
            .map_err(DbError::from)?;

        Ok(rows.into_iter().map(root_folder).collect::<Result<_, _>>()?)
    }

    async fn add_root_folder(&self, root: &RootFolder) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO root_folders (path, kind) VALUES (?, ?)")
            .bind(path_str(&root.path)?)
            .bind(root_kind_to_str(root.kind))
            .execute(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(())
    }

    async fn remove_root_folder(&self, path: &Path) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM root_folders WHERE path = ?")
            .bind(path_str(path)?)
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

        Ok(rows.into_iter().map(media_file).collect::<Result<_, _>>()?)
    }

    async fn import(&self, id: ImportId) -> Result<Option<Import>, StorageError> {
        let record: Option<ImportRecord> =
            sqlx::query_as("SELECT id, source, status, created_at FROM imports WHERE id = ?")
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
            "SELECT id, source, status, created_at FROM imports WHERE status = ? ORDER BY created_at, id",
        )
        .bind(import_status_to_str(status))
        .fetch_all(self.pool())
        .await
        .map_err(DbError::from)?;

        let mut imports = Vec::with_capacity(records.len());
        for record in records {
            imports.push(self.load_import(record).await?);
        }
        Ok(imports)
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
                .bind(path_str(path)?)
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
            "SELECT path, size, series_id, season, first_episode, last_episode, movie_id, confidence, skipped
             FROM import_rows WHERE import_id = ? ORDER BY position",
        )
        .bind(&record.id)
        .fetch_all(self.pool())
        .await?;

        Ok(Import {
            id: ImportId(codec::uuid(&record.id)?),
            source: PathBuf::from(record.source),
            status: import_status_from_str(&record.status)?,
            rows: rows.into_iter().map(import_row).collect::<Result<_, _>>()?,
            created_at: codec::timestamp(&record.created_at)?,
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
    .bind(path_str(&file.path)?)
    .bind(size_to_i64(file.size)?)
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
        "INSERT INTO imports (id, source, status, created_at) VALUES (?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET source = excluded.source, status = excluded.status",
    )
    .bind(&id)
    .bind(path_str(&import.source)?)
    .bind(import_status_to_str(import.status))
    .bind(import.created_at.to_string())
    .execute(&mut **tx)
    .await?;

    sqlx::query("DELETE FROM import_rows WHERE import_id = ?").bind(&id).execute(&mut **tx).await?;
    for (position, row) in (0_i64..).zip(&import.rows) {
        let target = TargetColumns::from(row.target);
        sqlx::query(
            "INSERT INTO import_rows (import_id, position, path, size, series_id, season, first_episode,
                                      last_episode, movie_id, confidence, skipped)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(position)
        .bind(path_str(&row.path)?)
        .bind(size_to_i64(row.size)?)
        .bind(target.series_id)
        .bind(target.season)
        .bind(target.first_episode)
        .bind(target.last_episode)
        .bind(target.movie_id)
        .bind(confidence_to_str(row.confidence))
        .bind(row.skipped)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn root_folder(row: RootFolderRow) -> Result<RootFolder, DbError> {
    let kind = match row.kind.as_str() {
        "series" => RootKind::Series,
        "movies" => RootKind::Movies,
        other => return Err(DbError::InvalidValue(format!("root folder kind {other:?}"))),
    };
    Ok(RootFolder { kind, path: PathBuf::from(row.path) })
}

fn media_file(row: MediaFileRow) -> Result<MediaFile, DbError> {
    Ok(MediaFile {
        id: MediaFileId(codec::uuid(&row.id)?),
        path: PathBuf::from(row.path),
        size: size_from_i64(row.size)?,
        target: row.target.into_target()?.ok_or_else(|| DbError::InvalidValue("media file without target".into()))?,
        added_at: codec::timestamp(&row.added_at)?,
    })
}

fn import_row(row: ImportRowRecord) -> Result<ImportRow, DbError> {
    Ok(ImportRow {
        path: PathBuf::from(row.path),
        size: size_from_i64(row.size)?,
        target: row.target.into_target()?,
        confidence: confidence_from_str(&row.confidence)?,
        skipped: row.skipped,
    })
}

impl From<Option<FileTarget>> for TargetColumns {
    fn from(target: Option<FileTarget>) -> Self {
        let empty = Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: None };
        match target {
            None => empty,
            Some(FileTarget::Episodes { series, span }) => Self {
                series_id: Some(series.to_string()),
                season: Some(span.season()),
                first_episode: Some(span.first()),
                last_episode: Some(span.last()),
                ..empty
            },
            Some(FileTarget::Movie(movie)) => Self { movie_id: Some(movie.to_string()), ..empty },
        }
    }
}

impl TargetColumns {
    fn into_target(self) -> Result<Option<FileTarget>, DbError> {
        match self {
            Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: None } => Ok(None),
            Self { series_id: None, season: None, first_episode: None, last_episode: None, movie_id: Some(movie) } => {
                Ok(Some(FileTarget::Movie(MovieId(codec::uuid(&movie)?))))
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
                Ok(Some(FileTarget::Episodes { series: SeriesId(codec::uuid(&series)?), span }))
            },
            _ => Err(DbError::InvalidValue("incomplete file target".into())),
        }
    }
}

fn path_str(path: &Path) -> Result<&str, DbError> {
    path.to_str().ok_or_else(|| DbError::InvalidValue(format!("path {} is not UTF-8", path.display())))
}

fn size_to_i64(size: u64) -> Result<i64, DbError> {
    i64::try_from(size).map_err(|_| DbError::InvalidValue(format!("size {size}")))
}

fn size_from_i64(size: i64) -> Result<u64, DbError> {
    u64::try_from(size).map_err(|_| DbError::InvalidValue(format!("size {size}")))
}

fn root_kind_to_str(kind: RootKind) -> &'static str {
    match kind {
        RootKind::Series => "series",
        RootKind::Movies => "movies",
    }
}

fn import_status_to_str(status: ImportStatus) -> &'static str {
    match status {
        ImportStatus::NeedsReview => "needs_review",
        ImportStatus::Done => "done",
    }
}

fn import_status_from_str(value: &str) -> Result<ImportStatus, DbError> {
    match value {
        "needs_review" => Ok(ImportStatus::NeedsReview),
        "done" => Ok(ImportStatus::Done),
        other => Err(DbError::InvalidValue(format!("import status {other:?}"))),
    }
}

fn confidence_to_str(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Unknown => "unknown",
        Confidence::Guess => "guess",
        Confidence::Certain => "certain",
    }
}

fn confidence_from_str(value: &str) -> Result<Confidence, DbError> {
    match value {
        "unknown" => Ok(Confidence::Unknown),
        "guess" => Ok(Confidence::Guess),
        "certain" => Ok(Confidence::Certain),
        other => Err(DbError::InvalidValue(format!("confidence {other:?}"))),
    }
}
