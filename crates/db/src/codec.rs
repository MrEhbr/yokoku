use std::{error::Error, fmt::Display, path::Path, str::FromStr};

use sqlx::{
    Decode, Encode, Sqlite, Type,
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentsBuffer, SqliteTypeInfo, SqliteValueRef},
};
use yokoku_domain::ExternalId;

use crate::DbError;

/// A TEXT column holding `T` in its `Display`/`FromStr` form.
pub(crate) struct Text<T>(pub(crate) T);

impl<T> Type<Sqlite> for Text<T> {
    fn type_info() -> SqliteTypeInfo {
        <str as Type<Sqlite>>::type_info()
    }
}

impl<T: Display> Encode<'_, Sqlite> for Text<T> {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode(self.0.to_string(), buf)
    }
}

impl<'r, T> Decode<'r, Sqlite> for Text<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(Self(<&str as Decode<Sqlite>>::decode(value)?.parse()?))
    }
}

/// An INTEGER bind value; encoding fails when `T` does not fit in `i64`.
pub(crate) struct Int<T>(pub(crate) T);

impl<T> Type<Sqlite> for Int<T> {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

impl<T: Copy> Encode<'_, Sqlite> for Int<T>
where
    i64: TryFrom<T>,
    <i64 as TryFrom<T>>::Error: Error + Send + Sync + 'static,
{
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        <i64 as Encode<Sqlite>>::encode(i64::try_from(self.0)?, buf)
    }
}

/// A TEXT bind value; encoding fails when the path is not UTF-8.
pub(crate) struct PathText<'a>(pub(crate) &'a Path);

impl Type<Sqlite> for PathText<'_> {
    fn type_info() -> SqliteTypeInfo {
        <str as Type<Sqlite>>::type_info()
    }
}

impl Encode<'_, Sqlite> for PathText<'_> {
    fn encode_by_ref(&self, buf: &mut SqliteArgumentsBuffer) -> Result<IsNull, BoxDynError> {
        let path = self.0.to_str().ok_or_else(|| format!("path {} is not UTF-8", self.0.display()))?;
        <&str as Encode<Sqlite>>::encode(path, buf)
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct SourceColumns {
    pub(crate) source_kind: String,
    pub(crate) source_id: u64,
}

impl From<ExternalId> for SourceColumns {
    fn from(source: ExternalId) -> Self {
        let (kind, id) = match source {
            ExternalId::Tmdb(id) => ("tmdb", id),
            ExternalId::Tvdb(id) => ("tvdb", id),
        };
        Self { source_kind: kind.to_owned(), source_id: id }
    }
}

impl TryFrom<SourceColumns> for ExternalId {
    type Error = DbError;

    fn try_from(columns: SourceColumns) -> Result<Self, Self::Error> {
        match columns.source_kind.as_str() {
            "tmdb" => Ok(Self::Tmdb(columns.source_id)),
            "tvdb" => Ok(Self::Tvdb(columns.source_id)),
            other => Err(DbError::InvalidValue(format!("source kind {other:?}"))),
        }
    }
}
