use jiff::{Timestamp, civil::Date};
use uuid::Uuid;
use yokoku_domain::{ExternalId, MediaFileId, Numbering, SourceStatus};

use crate::DbError;

pub(crate) fn source_columns(source: ExternalId) -> Result<(&'static str, i64), DbError> {
    let (kind, id) = match source {
        ExternalId::Tmdb(id) => ("tmdb", id),
        ExternalId::Tvdb(id) => ("tvdb", id),
    };
    Ok((kind, source_id_to_i64(id)?))
}

pub(crate) fn source_from_columns(kind: &str, id: i64) -> Result<ExternalId, DbError> {
    let id = source_id_from_i64(id)?;
    match kind {
        "tmdb" => Ok(ExternalId::Tmdb(id)),
        "tvdb" => Ok(ExternalId::Tvdb(id)),
        other => Err(DbError::InvalidValue(format!("source kind {other:?}"))),
    }
}

pub(crate) fn source_id_to_i64(id: u64) -> Result<i64, DbError> {
    i64::try_from(id).map_err(|_| DbError::InvalidValue(format!("source id {id} out of range")))
}

pub(crate) fn source_id_from_i64(id: i64) -> Result<u64, DbError> {
    u64::try_from(id).map_err(|_| DbError::InvalidValue(format!("source id {id}")))
}

pub(crate) fn source_status_to_str(status: SourceStatus) -> &'static str {
    match status {
        SourceStatus::Returning => "returning",
        SourceStatus::Planned => "planned",
        SourceStatus::InProduction => "in_production",
        SourceStatus::Pilot => "pilot",
        SourceStatus::Ended => "ended",
        SourceStatus::Canceled => "canceled",
        SourceStatus::Unknown => "unknown",
    }
}

pub(crate) fn source_status_from_str(value: &str) -> Result<SourceStatus, DbError> {
    Ok(match value {
        "returning" => SourceStatus::Returning,
        "planned" => SourceStatus::Planned,
        "in_production" => SourceStatus::InProduction,
        "pilot" => SourceStatus::Pilot,
        "ended" => SourceStatus::Ended,
        "canceled" => SourceStatus::Canceled,
        "unknown" => SourceStatus::Unknown,
        other => return Err(DbError::InvalidValue(format!("source status {other:?}"))),
    })
}

pub(crate) fn numbering_to_str(numbering: Numbering) -> &'static str {
    match numbering {
        Numbering::Standard => "standard",
        Numbering::Absolute => "absolute",
    }
}

pub(crate) fn numbering_from_str(value: &str) -> Result<Numbering, DbError> {
    match value {
        "standard" => Ok(Numbering::Standard),
        "absolute" => Ok(Numbering::Absolute),
        other => Err(DbError::InvalidValue(format!("numbering {other:?}"))),
    }
}

pub(crate) fn uuid(value: &str) -> Result<Uuid, DbError> {
    value.parse().map_err(|_| DbError::InvalidValue(format!("id {value:?}")))
}

pub(crate) fn timestamp(value: &str) -> Result<Timestamp, DbError> {
    value.parse().map_err(|_| DbError::InvalidValue(format!("timestamp {value:?}")))
}

pub(crate) fn date(value: Option<&str>) -> Result<Option<Date>, DbError> {
    value.map(|value| value.parse().map_err(|_| DbError::InvalidValue(format!("date {value:?}")))).transpose()
}

pub(crate) fn file_id(value: Option<&str>) -> Result<Option<MediaFileId>, DbError> {
    value.map(|value| uuid(value).map(MediaFileId)).transpose()
}

pub(crate) fn revision(value: i64) -> Result<u64, DbError> {
    u64::try_from(value).map_err(|_| DbError::InvalidValue(format!("revision {value}")))
}

pub(crate) fn revision_to_i64(revision: u64) -> Result<i64, DbError> {
    i64::try_from(revision).map_err(|_| DbError::InvalidValue(format!("revision {revision}")))
}
