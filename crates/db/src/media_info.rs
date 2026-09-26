use std::time::Duration;

use yokoku_domain::MediaFileId;
use yokoku_media::{AudioStream, MediaInfo, SubtitleStream, VideoStream};

use crate::{Database, DbError, codec::Int};

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

impl Database {
    pub(crate) async fn load_media_info(&self, file: MediaFileId) -> Result<Option<MediaInfo>, DbError> {
        let file = file.to_string();
        let Some(row) = sqlx::query_as::<_, InfoRow>(
            "SELECT duration_ms, video_codec, width, height FROM media_info WHERE file_id = ?",
        )
        .bind(&file)
        .fetch_optional(self.pool())
        .await?
        else {
            return Ok(None);
        };
        let streams: Vec<StreamRow> = sqlx::query_as(
            "SELECT kind, codec, language, channels, forced FROM media_streams WHERE file_id = ? ORDER BY position",
        )
        .bind(&file)
        .fetch_all(self.pool())
        .await?;

        let video = match (row.video_codec, row.width, row.height) {
            (Some(codec), Some(width), Some(height)) => Some(VideoStream { codec, width, height }),
            _ => None,
        };
        let mut info =
            MediaInfo { duration: row.duration_ms.map(Duration::from_millis), video, ..MediaInfo::default() };
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
        Ok(Some(info))
    }

    pub(crate) async fn store_media_info(&self, file: MediaFileId, info: &MediaInfo) -> Result<(), DbError> {
        let file = file.to_string();
        let mut tx = self.begin().await?;
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
