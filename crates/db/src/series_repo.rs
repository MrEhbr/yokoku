use std::collections::BTreeMap;

use async_trait::async_trait;
use yokoku_domain::{Episode, ExternalId, Season, Series, SeriesId};
use yokoku_events::Event;
use yokoku_library::ports::{SeriesRepo, StorageError};

use crate::{Database, DbError, codec};

#[derive(sqlx::FromRow)]
struct SeriesRow {
    id: String,
    source_kind: String,
    source_id: i64,
    title: String,
    original_title: String,
    year: Option<i16>,
    poster_path: Option<String>,
    source_status: String,
    numbering: String,
    monitored: bool,
    added_at: String,
    refreshed_at: String,
}

#[derive(sqlx::FromRow)]
struct SeasonRow {
    number: u16,
    monitored: bool,
}

#[derive(sqlx::FromRow)]
struct EpisodeRow {
    id: String,
    season_number: u16,
    source_id: i64,
    number: u16,
    title: String,
    air_date: Option<String>,
    monitored: bool,
    has_file: bool,
}

#[async_trait]
impl SeriesRepo for Database {
    async fn get(&self, id: SeriesId) -> Result<Option<Series>, StorageError> {
        Ok(self.load_series(id).await?)
    }

    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Series>, StorageError> {
        let (kind, source_id) = codec::source_columns(source)?;
        let id: Option<String> = sqlx::query_scalar("SELECT id FROM series WHERE source_kind = ? AND source_id = ?")
            .bind(kind)
            .bind(source_id)
            .fetch_optional(self.pool())
            .await
            .map_err(DbError::from)?;

        match id {
            Some(id) => Ok(self.load_series(SeriesId(codec::uuid(&id)?)).await?),
            None => Ok(None),
        }
    }

    async fn ids(&self) -> Result<Vec<SeriesId>, StorageError> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM series ORDER BY id")
            .fetch_all(self.pool())
            .await
            .map_err(DbError::from)?;

        Ok(ids.iter().map(|id| codec::uuid(id).map(SeriesId)).collect::<Result<_, _>>()?)
    }

    async fn save(&self, series: &Series, events: &[Event]) -> Result<(), StorageError> {
        Ok(self.save_series(series, events).await?)
    }

    async fn remove(&self, id: SeriesId, events: &[Event]) -> Result<(), StorageError> {
        let mut tx = self.begin().await?;
        sqlx::query("DELETE FROM series WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(DbError::from)?;
        Ok(self.commit(tx, events).await?)
    }
}

impl Database {
    async fn load_series(&self, id: SeriesId) -> Result<Option<Series>, DbError> {
        let id = id.to_string();
        let Some(row) = sqlx::query_as::<_, SeriesRow>(
            "SELECT id, source_kind, source_id, title, original_title, year, poster_path, source_status, numbering,
                    monitored, added_at, refreshed_at
             FROM series WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(self.pool())
        .await?
        else {
            return Ok(None);
        };

        let seasons: Vec<SeasonRow> =
            sqlx::query_as("SELECT number, monitored FROM seasons WHERE series_id = ? ORDER BY number")
                .bind(&id)
                .fetch_all(self.pool())
                .await?;
        let episodes: Vec<EpisodeRow> = sqlx::query_as(
            "SELECT id, season_number, source_id, number, title, air_date, monitored, has_file
             FROM episodes WHERE series_id = ? ORDER BY season_number, number",
        )
        .bind(&id)
        .fetch_all(self.pool())
        .await?;

        Ok(Some(assemble(row, seasons, episodes)?))
    }

    async fn save_series(&self, series: &Series, events: &[Event]) -> Result<(), DbError> {
        let id = series.id.to_string();
        let (source_kind, source_id) = codec::source_columns(series.source)?;
        let mut tx = self.begin().await?;

        sqlx::query(
            "INSERT INTO series (id, source_kind, source_id, title, original_title, year, poster_path, source_status,
                                 numbering, monitored, added_at, refreshed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 title = excluded.title, original_title = excluded.original_title, year = excluded.year,
                 poster_path = excluded.poster_path, source_status = excluded.source_status,
                 numbering = excluded.numbering, monitored = excluded.monitored,
                 refreshed_at = excluded.refreshed_at",
        )
        .bind(&id)
        .bind(source_kind)
        .bind(source_id)
        .bind(&series.title)
        .bind(&series.original_title)
        .bind(series.year)
        .bind(&series.poster_path)
        .bind(codec::source_status_to_str(series.source_status))
        .bind(codec::numbering_to_str(series.numbering))
        .bind(series.monitored)
        .bind(series.added_at.to_string())
        .bind(series.refreshed_at.to_string())
        .execute(&mut *tx)
        .await?;

        for season in &series.seasons {
            sqlx::query(
                "INSERT INTO seasons (series_id, number, monitored) VALUES (?, ?, ?)
                 ON CONFLICT (series_id, number) DO UPDATE SET monitored = excluded.monitored",
            )
            .bind(&id)
            .bind(season.number)
            .bind(season.monitored)
            .execute(&mut *tx)
            .await?;

            for episode in &season.episodes {
                sqlx::query(
                    "INSERT INTO episodes (id, series_id, season_number, source_id, number, title, air_date,
                                           monitored, has_file)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT (id) DO UPDATE SET
                         season_number = excluded.season_number, number = excluded.number,
                         title = excluded.title, air_date = excluded.air_date,
                         monitored = excluded.monitored, has_file = excluded.has_file",
                )
                .bind(episode.id.to_string())
                .bind(&id)
                .bind(season.number)
                .bind(codec::source_id_to_i64(episode.source_id)?)
                .bind(episode.number)
                .bind(&episode.title)
                .bind(episode.air_date.map(|date| date.to_string()))
                .bind(episode.monitored)
                .bind(episode.has_file)
                .execute(&mut *tx)
                .await?;
            }
        }

        let episode_ids: Vec<String> = series.episodes().map(|episode| episode.id.to_string()).collect();
        sqlx::query("DELETE FROM episodes WHERE series_id = ? AND id NOT IN (SELECT value FROM json_each(?))")
            .bind(&id)
            .bind(serde_json::to_string(&episode_ids)?)
            .execute(&mut *tx)
            .await?;

        let season_numbers: Vec<u16> = series.seasons.iter().map(|season| season.number).collect();
        sqlx::query("DELETE FROM seasons WHERE series_id = ? AND number NOT IN (SELECT value FROM json_each(?))")
            .bind(&id)
            .bind(serde_json::to_string(&season_numbers)?)
            .execute(&mut *tx)
            .await?;

        self.commit(tx, events).await
    }
}

fn assemble(row: SeriesRow, seasons: Vec<SeasonRow>, episodes: Vec<EpisodeRow>) -> Result<Series, DbError> {
    let mut episodes_by_season: BTreeMap<u16, Vec<Episode>> = BTreeMap::new();
    for episode in episodes {
        episodes_by_season.entry(episode.season_number).or_default().push(Episode {
            id: yokoku_domain::EpisodeId(codec::uuid(&episode.id)?),
            source_id: codec::source_id_from_i64(episode.source_id)?,
            number: episode.number,
            title: episode.title,
            air_date: codec::date(episode.air_date.as_deref())?,
            monitored: episode.monitored,
            has_file: episode.has_file,
        });
    }

    let seasons = seasons
        .into_iter()
        .map(|season| Season {
            number: season.number,
            monitored: season.monitored,
            episodes: episodes_by_season.remove(&season.number).unwrap_or_default(),
        })
        .collect();

    Ok(Series {
        id: SeriesId(codec::uuid(&row.id)?),
        source: codec::source_from_columns(&row.source_kind, row.source_id)?,
        title: row.title,
        original_title: row.original_title,
        year: row.year,
        poster_path: row.poster_path,
        source_status: codec::source_status_from_str(&row.source_status)?,
        numbering: codec::numbering_from_str(&row.numbering)?,
        monitored: row.monitored,
        seasons,
        added_at: codec::timestamp(&row.added_at)?,
        refreshed_at: codec::timestamp(&row.refreshed_at)?,
    })
}
