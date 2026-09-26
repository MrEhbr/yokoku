use std::{collections::BTreeMap, path::PathBuf};

use async_trait::async_trait;
use jiff::{Timestamp, civil::Date};
use sqlx::types::Json;
use yokoku_domain::{
    Episode, EpisodeId, ExternalId, ItemFolder, MediaFileId, Numbering, Season, Series, SeriesId, SourceStatus,
    StorageError,
};
use yokoku_events::Event;
use yokoku_library::ports::SeriesRepo;

use crate::{
    Database, DbError,
    codec::{Int, PathText, SourceColumns, Text},
};

#[derive(sqlx::FromRow)]
struct SeriesRow {
    id: Text<SeriesId>,
    #[sqlx(flatten)]
    source: SourceColumns,
    title: String,
    original_title: String,
    alternate_titles: Json<Vec<String>>,
    year: Option<i16>,
    poster_path: Option<String>,
    source_status: Text<SourceStatus>,
    numbering: Text<Numbering>,
    root: String,
    folder: String,
    monitored: bool,
    added_at: Text<Timestamp>,
    refreshed_at: Text<Timestamp>,
    revision: u64,
}

#[derive(sqlx::FromRow)]
struct SeasonRow {
    number: u16,
    monitored: bool,
}

#[derive(sqlx::FromRow)]
struct EpisodeRow {
    id: Text<EpisodeId>,
    season_number: u16,
    source_id: u64,
    number: u16,
    title: String,
    air_date: Option<Text<Date>>,
    monitored: bool,
    file_id: Option<Text<MediaFileId>>,
}

#[async_trait]
impl SeriesRepo for Database {
    async fn get(&self, id: SeriesId) -> Result<Option<Series>, StorageError> {
        Ok(self.load_series(id).await?)
    }

    async fn find_by_source(&self, source: ExternalId) -> Result<Option<Series>, StorageError> {
        let source = SourceColumns::from(source);
        let id: Option<Text<SeriesId>> =
            sqlx::query_scalar("SELECT id FROM series WHERE source_kind = ? AND source_id = ?")
                .bind(source.source_kind)
                .bind(Int(source.source_id))
                .fetch_optional(self.pool())
                .await
                .map_err(DbError::from)?;

        match id {
            Some(id) => Ok(self.load_series(id.0).await?),
            None => Ok(None),
        }
    }

    async fn find_by_folder(&self, folder: &ItemFolder) -> Result<Option<SeriesId>, StorageError> {
        let id: Option<Text<SeriesId>> = sqlx::query_scalar("SELECT id FROM series WHERE root = ? AND folder = ?")
            .bind(PathText(&folder.root))
            .bind(&folder.name)
            .fetch_optional(self.pool())
            .await
            .map_err(DbError::from)?;
        Ok(id.map(|id| id.0))
    }

    async fn ids(&self) -> Result<Vec<SeriesId>, StorageError> {
        Ok(self.series_ids().await?)
    }

    async fn save(&self, series: &mut Series, events: &[Event]) -> Result<(), StorageError> {
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
    pub(crate) async fn series_ids(&self) -> Result<Vec<SeriesId>, DbError> {
        let ids: Vec<Text<SeriesId>> =
            sqlx::query_scalar("SELECT id FROM series ORDER BY id").fetch_all(self.pool()).await?;
        Ok(ids.into_iter().map(|id| id.0).collect())
    }

    pub(crate) async fn load_series(&self, id: SeriesId) -> Result<Option<Series>, DbError> {
        let id = id.to_string();
        let Some(row) = sqlx::query_as::<_, SeriesRow>(
            "SELECT id, source_kind, source_id, title, original_title, alternate_titles, year, poster_path, source_status,
                    numbering, root, folder, monitored, added_at, refreshed_at, revision
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
            "SELECT id, season_number, source_id, number, title, air_date, monitored, file_id
             FROM episodes WHERE series_id = ? ORDER BY season_number, number",
        )
        .bind(&id)
        .fetch_all(self.pool())
        .await?;

        Ok(Some(row.into_series(seasons, episodes)?))
    }

    async fn save_series(&self, series: &mut Series, events: &[Event]) -> Result<(), DbError> {
        let id = series.id.to_string();
        let source = SourceColumns::from(series.source);
        let mut tx = self.begin_save("series", &id, series.revision).await?;

        sqlx::query(
            "INSERT INTO series (id, source_kind, source_id, title, original_title, alternate_titles, year, poster_path,
                                 source_status, numbering, root, folder, monitored, added_at, refreshed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET
                 title = excluded.title, original_title = excluded.original_title,
                 alternate_titles = excluded.alternate_titles, year = excluded.year,
                 poster_path = excluded.poster_path, source_status = excluded.source_status,
                 numbering = excluded.numbering, monitored = excluded.monitored,
                 refreshed_at = excluded.refreshed_at",
        )
        .bind(&id)
        .bind(source.source_kind)
        .bind(Int(source.source_id))
        .bind(&series.title)
        .bind(&series.original_title)
        .bind(Json(&series.alternate_titles))
        .bind(series.year)
        .bind(&series.poster_path)
        .bind(series.source_status.as_str())
        .bind(series.numbering.as_str())
        .bind(PathText(&series.folder.root))
        .bind(&series.folder.name)
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
                                           monitored, file_id)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT (id) DO UPDATE SET
                         season_number = excluded.season_number, number = excluded.number,
                         title = excluded.title, air_date = excluded.air_date,
                         monitored = excluded.monitored, file_id = excluded.file_id",
                )
                .bind(episode.id.to_string())
                .bind(&id)
                .bind(season.number)
                .bind(Int(episode.source_id))
                .bind(episode.number)
                .bind(&episode.title)
                .bind(episode.air_date.map(|date| date.to_string()))
                .bind(episode.monitored)
                .bind(episode.file.map(|file| file.to_string()))
                .execute(&mut *tx)
                .await?;
            }
        }

        let episode_ids: Vec<String> = series.episodes().map(|episode| episode.id.to_string()).collect();
        sqlx::query("DELETE FROM episodes WHERE series_id = ? AND id NOT IN (SELECT value FROM json_each(?))")
            .bind(&id)
            .bind(Json(&episode_ids))
            .execute(&mut *tx)
            .await?;

        let season_numbers: Vec<u16> = series.seasons.iter().map(|season| season.number).collect();
        sqlx::query("DELETE FROM seasons WHERE series_id = ? AND number NOT IN (SELECT value FROM json_each(?))")
            .bind(&id)
            .bind(Json(&season_numbers))
            .execute(&mut *tx)
            .await?;

        self.commit(tx, events).await?;
        series.revision += 1;
        Ok(())
    }
}

impl From<EpisodeRow> for Episode {
    fn from(episode: EpisodeRow) -> Self {
        Self {
            id: episode.id.0,
            source_id: episode.source_id,
            number: episode.number,
            title: episode.title,
            air_date: episode.air_date.map(|date| date.0),
            monitored: episode.monitored,
            file: episode.file_id.map(|file| file.0),
        }
    }
}

impl SeriesRow {
    fn into_series(self, seasons: Vec<SeasonRow>, episodes: Vec<EpisodeRow>) -> Result<Series, DbError> {
        let mut episodes_by_season: BTreeMap<u16, Vec<Episode>> = BTreeMap::new();
        for episode in episodes {
            episodes_by_season.entry(episode.season_number).or_default().push(episode.into());
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
            id: self.id.0,
            source: self.source.try_into()?,
            title: self.title,
            original_title: self.original_title,
            alternate_titles: self.alternate_titles.0,
            year: self.year,
            poster_path: self.poster_path,
            source_status: self.source_status.0,
            numbering: self.numbering.0,
            folder: ItemFolder { root: PathBuf::from(self.root), name: self.folder },
            monitored: self.monitored,
            seasons,
            added_at: self.added_at.0,
            refreshed_at: self.refreshed_at.0,
            revision: self.revision,
        })
    }
}
