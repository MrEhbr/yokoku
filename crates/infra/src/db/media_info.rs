use std::{collections::HashMap, time::Duration};

use yokoku_core::media::{AudioStream, MediaInfo, SubtitleStream, VideoStream};
use yokoku_domain::{ItemId, MediaFileId};

use crate::db::{
    Database, DbError,
    codec::{Int, Text},
};

#[derive(sqlx::FromRow)]
struct InfoRow {
    duration_ms: Option<u64>,
    video_codec: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(sqlx::FromRow)]
struct StreamRow {
    kind: String,
    codec: String,
    language: Option<String>,
    channels: Option<u16>,
    forced: bool,
}

#[derive(sqlx::FromRow)]
struct FileInfoRow {
    file_id: Text<MediaFileId>,
    #[sqlx(flatten)]
    info: InfoRow,
}

#[derive(sqlx::FromRow)]
struct FileStreamRow {
    file_id: Text<MediaFileId>,
    #[sqlx(flatten)]
    stream: StreamRow,
}

fn media_info(row: InfoRow, streams: Vec<StreamRow>) -> Result<MediaInfo, DbError> {
    let video = match (row.video_codec, row.width, row.height) {
        (Some(codec), Some(width), Some(height)) => Some(VideoStream { codec, width, height }),
        _ => None,
    };
    let mut info = MediaInfo { duration: row.duration_ms.map(Duration::from_millis), video, ..MediaInfo::default() };
    for stream in streams {
        match (stream.kind.as_str(), stream.channels) {
            ("audio", Some(channels)) => {
                info.audio.push(AudioStream { codec: stream.codec, language: stream.language, channels });
            },
            ("subtitle", None) => info.subtitles.push(SubtitleStream {
                codec: stream.codec,
                language: stream.language,
                forced: stream.forced,
            }),
            (kind, _) => return Err(DbError::InvalidValue(format!("stream kind {kind}"))),
        }
    }
    Ok(info)
}

impl Database {
    pub(crate) async fn load_media_info_of(&self, item: ItemId) -> Result<HashMap<MediaFileId, MediaInfo>, DbError> {
        let (series, movie) = (item.series().map(Text), item.movie().map(Text));
        let rows: Vec<FileInfoRow> = sqlx::query_as(
            "SELECT i.file_id, i.duration_ms, i.video_codec, i.width, i.height
             FROM media_info i JOIN media_files f ON f.id = i.file_id WHERE f.series_id = ? OR f.movie_id = ?",
        )
        .bind(&series)
        .bind(&movie)
        .fetch_all(self.pool())
        .await?;
        let streams: Vec<FileStreamRow> = sqlx::query_as(
            "SELECT s.file_id, s.kind, s.codec, s.language, s.channels, s.forced
             FROM media_streams s JOIN media_files f ON f.id = s.file_id WHERE f.series_id = ? OR f.movie_id = ?
             ORDER BY s.file_id, s.position",
        )
        .bind(&series)
        .bind(&movie)
        .fetch_all(self.pool())
        .await?;

        let mut streams_of: HashMap<MediaFileId, Vec<StreamRow>> = HashMap::new();
        for stream in streams {
            streams_of.entry(stream.file_id.0).or_default().push(stream.stream);
        }
        rows.into_iter()
            .map(|row| {
                let file = row.file_id.0;
                Ok((file, media_info(row.info, streams_of.remove(&file).unwrap_or_default())?))
            })
            .collect()
    }

    pub(crate) async fn store_media_info(&self, file: MediaFileId, info: &MediaInfo) -> Result<(), DbError> {
        let file = file.to_string();
        let mut tx = self.pool().begin().await?;
        sqlx::query("DELETE FROM media_info WHERE file_id = ?").bind(&file).execute(&mut *tx).await?;
        let inserted = sqlx::query(
            "INSERT INTO media_info (file_id, duration_ms, video_codec, width, height)
             SELECT id, ?, ?, ?, ? FROM media_files WHERE id = ?",
        )
        .bind(info.duration.map(|duration| Int(duration.as_millis())))
        .bind(info.video.as_ref().map(|video| &video.codec))
        .bind(info.video.as_ref().map(|video| video.width))
        .bind(info.video.as_ref().map(|video| video.height))
        .bind(&file)
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() == 0 {
            return Ok(());
        }

        let audio =
            info.audio.iter().map(|audio| ("audio", &audio.codec, &audio.language, Some(audio.channels), false));
        let subtitles = info
            .subtitles
            .iter()
            .map(|subtitle| ("subtitle", &subtitle.codec, &subtitle.language, None, subtitle.forced));
        for (position, (kind, codec, language, channels, forced)) in (0_i64..).zip(audio.chain(subtitles)) {
            sqlx::query(
                "INSERT INTO media_streams (file_id, position, kind, codec, language, channels, forced)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&file)
            .bind(position)
            .bind(kind)
            .bind(codec)
            .bind(language)
            .bind(channels)
            .bind(forced)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
