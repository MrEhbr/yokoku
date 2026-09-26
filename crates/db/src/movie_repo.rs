use async_trait::async_trait;
use yokoku_domain::{ExternalId, Movie, MovieId, Releases};
use yokoku_events::Event;
use yokoku_library::ports::{MovieRepo, StorageError};

use crate::{Database, DbError, codec};

#[derive(sqlx::FromRow)]
struct MovieRow {
    id: String,
    source_kind: String,
    source_id: i64,
    title: String,
    original_title: String,
    year: Option<i16>,
    poster_path: Option<String>,
    cinema_date: Option<String>,
    digital_date: Option<String>,
    physical_date: Option<String>,
    monitored: bool,
    file_id: Option<String>,
    added_at: String,
    refreshed_at: String,
}

#[async_trait]
impl MovieRepo for Database {
    async fn get(&self, id: MovieId) -> Result<Option<Movie>, StorageError> {
        Ok(self.load_movie(id).await?)
    }

    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Movie>, StorageError> {
        let (kind, source_id) = codec::source_columns(source)?;
        let id: Option<String> = sqlx::query_scalar("SELECT id FROM movies WHERE source_kind = ? AND source_id = ?")
            .bind(kind)
            .bind(source_id)
            .fetch_optional(self.pool())
            .await
            .map_err(DbError::from)?;

        match id {
            Some(id) => Ok(self.load_movie(MovieId(codec::uuid(&id)?)).await?),
            None => Ok(None),
        }
    }

    async fn ids(&self) -> Result<Vec<MovieId>, StorageError> {
        Ok(self.movie_ids().await?)
    }

    async fn save(&self, movie: &Movie, events: &[Event]) -> Result<(), StorageError> {
        Ok(self.save_movie(movie, events).await?)
    }

    async fn remove(&self, id: MovieId, events: &[Event]) -> Result<(), StorageError> {
        let mut tx = self.begin().await?;
        sqlx::query("DELETE FROM movies WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(DbError::from)?;
        Ok(self.commit(tx, events).await?)
    }
}

impl Database {
    async fn save_movie(&self, movie: &Movie, events: &[Event]) -> Result<(), DbError> {
        let (source_kind, source_id) = codec::source_columns(movie.source)?;
        let date = |date: Option<jiff::civil::Date>| date.map(|date| date.to_string());
        let mut tx = self.begin().await?;

        sqlx::query(
            "INSERT INTO movies (id, source_kind, source_id, title, original_title, year, poster_path, cinema_date,
                                 digital_date, physical_date, monitored, file_id, added_at, refreshed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 title = excluded.title, original_title = excluded.original_title, year = excluded.year,
                 poster_path = excluded.poster_path, cinema_date = excluded.cinema_date,
                 digital_date = excluded.digital_date, physical_date = excluded.physical_date,
                 monitored = excluded.monitored, file_id = excluded.file_id,
                 refreshed_at = excluded.refreshed_at",
        )
        .bind(movie.id.to_string())
        .bind(source_kind)
        .bind(source_id)
        .bind(&movie.title)
        .bind(&movie.original_title)
        .bind(movie.year)
        .bind(&movie.poster_path)
        .bind(date(movie.releases.cinema))
        .bind(date(movie.releases.digital))
        .bind(date(movie.releases.physical))
        .bind(movie.monitored)
        .bind(movie.file.map(|file| file.to_string()))
        .bind(movie.added_at.to_string())
        .bind(movie.refreshed_at.to_string())
        .execute(&mut *tx)
        .await?;

        self.commit(tx, events).await
    }

    pub(crate) async fn load_movie(&self, id: MovieId) -> Result<Option<Movie>, DbError> {
        let row: Option<MovieRow> = sqlx::query_as(
            "SELECT id, source_kind, source_id, title, original_title, year, poster_path, cinema_date, digital_date,
                    physical_date, monitored, file_id, added_at, refreshed_at
             FROM movies WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool())
        .await?;

        row.map(movie).transpose()
    }

    pub(crate) async fn movie_ids(&self) -> Result<Vec<MovieId>, DbError> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM movies ORDER BY id").fetch_all(self.pool()).await?;
        ids.iter().map(|id| codec::uuid(id).map(MovieId)).collect()
    }
}

fn movie(row: MovieRow) -> Result<Movie, DbError> {
    Ok(Movie {
        id: MovieId(codec::uuid(&row.id)?),
        source: codec::source_from_columns(&row.source_kind, row.source_id)?,
        title: row.title,
        original_title: row.original_title,
        year: row.year,
        poster_path: row.poster_path,
        releases: Releases {
            cinema: codec::date(row.cinema_date.as_deref())?,
            digital: codec::date(row.digital_date.as_deref())?,
            physical: codec::date(row.physical_date.as_deref())?,
        },
        monitored: row.monitored,
        file: codec::file_id(row.file_id.as_deref())?,
        added_at: codec::timestamp(&row.added_at)?,
        refreshed_at: codec::timestamp(&row.refreshed_at)?,
    })
}
