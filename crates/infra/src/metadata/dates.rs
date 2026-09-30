//! Lenient `deserialize_with` readers for the dates metadata sources send; an empty, missing or
//! unreadable value is `None`.

use jiff::civil::Date;
use serde::{Deserialize, Deserializer};

/// `2021-10-22` or `2021-10-22T00:00:00.000Z`.
pub(crate) fn date<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Date>, D::Error> {
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.as_deref().and_then(|value| value.get(..10)?.parse().ok()))
}

/// `2021`.
pub(crate) fn year<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<i16>, D::Error> {
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.as_deref().and_then(|value| value.get(..4)?.parse().ok()))
}
