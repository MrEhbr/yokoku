use std::path::PathBuf;

use async_trait::async_trait;
use jiff::{Timestamp, civil::Date};
use sqlx::types::Json;
use yokoku_domain::{ExternalId, ItemFolder, MediaFileId, Movie, MovieId, Releases, StorageError};
use yokoku_library::ports::MovieRepo;

use crate::{
    Database, DbError,
    codec::{Int, PathText, SourceColumns, Text},
};

#[derive(sqlx::FromRow)]
struct MovieRow {
    id: Text<MovieId>,
    #[sqlx(flatten)]
    source: SourceColumns,
    title: String,
    original_title: String,
    alternate_titles: Json<Vec<String>>,
    year: Option<i16>,
    poster_path: Option<String>,
    cinema_date: Option<Text<Date>>,
    digital_date: Option<Text<Date>>,
    physical_date: Option<Text<Date>>,
    root: String,
    folder: String,
    monitored: bool,
    file_id: Option<Text<MediaFileId>>,
    added_at: Text<Timestamp>,
    refreshed_at: Text<Timestamp>,
    revision: u64,
}

#[async_trait]
impl MovieRepo for Database {
    async fn get(&self, id: MovieId) -> Result<Option<Movie>, StorageError> {
        Ok(self.load_movie(id).await?)
    }

    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Movie>, StorageError> {
        let source = SourceColumns::from(source);
        let id: Option<Text<MovieId>> =
            sqlx::query_scalar("SELECT id FROM movies WHERE source_kind = ? AND source_id = ?")
                .bind(source.source_kind)
                .bind(Int(source.source_id))
                .fetch_optional(self.pool())
                .await
                .map_err(DbError::from)?;

        match id {
            Some(id) => Ok(self.load_movie(id.0).await?),
            None => Ok(None),
        }
    }

    async fn find_by_folder(&self, folder: &ItemFolder) -> Result<Option<MovieId>, StorageError> {
        let id: Option<Text<MovieId>> = sqlx::query_scalar("SELECT id FROM movies WHERE root = ? AND folder = ?")
            .bind(PathText(&folder.root))
            .bind(&folder.name)
            .fetch_optional(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(id.map(|id| id.0))
    }

    async fn ids(&self) -> Result<Vec<MovieId>, StorageError> {
        Ok(self.movie_ids().await?)
    }

    async fn save(&self, movie: &mut Movie) -> Result<(), StorageError> {
        Ok(self.save_movie(movie).await?)
    }

    async fn remove(&self, id: MovieId) -> Result<(), StorageError> {
        let mut tx = self.begin().await?;
        sqlx::query("DELETE FROM movies WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(DbError::from)?;
        Ok(self.commit(tx, &[]).await?)
    }
}

impl Database {
    async fn save_movie(&self, movie: &mut Movie) -> Result<(), DbError> {
        let source = SourceColumns::from(movie.source);
        let date = |date: Option<Date>| date.map(|date| date.to_string());
        let mut tx = self.begin_save("movies", &movie.id.to_string(), movie.revision).await?;

        sqlx::query(
            "INSERT INTO movies (id, source_kind, source_id, title, original_title, alternate_titles, year, poster_path,
                                 cinema_date, digital_date, physical_date, root, folder, monitored, file_id, added_at,
                                 refreshed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 title = excluded.title, original_title = excluded.original_title,
                 alternate_titles = excluded.alternate_titles, year = excluded.year,
                 poster_path = excluded.poster_path, cinema_date = excluded.cinema_date,
                 digital_date = excluded.digital_date, physical_date = excluded.physical_date,
                 monitored = excluded.monitored, file_id = excluded.file_id,
                 refreshed_at = excluded.refreshed_at",
        )
        .bind(movie.id.to_string())
        .bind(source.source_kind)
        .bind(Int(source.source_id))
        .bind(&movie.title)
        .bind(&movie.original_title)
        .bind(Json(&movie.alternate_titles))
        .bind(movie.year)
        .bind(&movie.poster_path)
        .bind(date(movie.releases.cinema))
        .bind(date(movie.releases.digital))
        .bind(date(movie.releases.physical))
        .bind(PathText(&movie.folder.root))
        .bind(&movie.folder.name)
        .bind(movie.monitored)
        .bind(movie.file.map(|file| file.to_string()))
        .bind(movie.added_at.to_string())
        .bind(movie.refreshed_at.to_string())
        .execute(&mut *tx)
        .await?;

        self.commit(tx, &[]).await?;
        movie.revision += 1;
        Ok(())
    }

    pub(crate) async fn load_movie(&self, id: MovieId) -> Result<Option<Movie>, DbError> {
        let row: Option<MovieRow> = sqlx::query_as(
            "SELECT id, source_kind, source_id, title, original_title, alternate_titles, year, poster_path,
                    cinema_date, digital_date, physical_date, root, folder, monitored, file_id, added_at, refreshed_at,
                    revision
             FROM movies WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool())
        .await?;

        row.map(Movie::try_from).transpose()
    }

    pub(crate) async fn movie_ids(&self) -> Result<Vec<MovieId>, DbError> {
        let ids: Vec<Text<MovieId>> =
            sqlx::query_scalar("SELECT id FROM movies ORDER BY id").fetch_all(self.pool()).await?;
        Ok(ids.into_iter().map(|id| id.0).collect())
    }
}

impl TryFrom<MovieRow> for Movie {
    type Error = DbError;

    fn try_from(row: MovieRow) -> Result<Self, Self::Error> {
        Ok(Movie {
            id: row.id.0,
            source: row.source.try_into()?,
            title: row.title,
            original_title: row.original_title,
            alternate_titles: row.alternate_titles.0,
            year: row.year,
            poster_path: row.poster_path,
            releases: Releases {
                cinema: row.cinema_date.map(|date| date.0),
                digital: row.digital_date.map(|date| date.0),
                physical: row.physical_date.map(|date| date.0),
            },
            folder: ItemFolder { root: PathBuf::from(row.root), name: row.folder },
            monitored: row.monitored,
            file: row.file_id.map(|file| file.0),
            added_at: row.added_at.0,
            refreshed_at: row.refreshed_at.0,
            revision: row.revision,
        })
    }
}
