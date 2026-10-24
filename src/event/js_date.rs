//! Dates the way `JSON.stringify` writes them: ISO 8601, milliseconds, `Z`.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serializer};

pub(super) fn serialize<S: Serializer>(
    date: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&date.to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error> {
    let text = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&text)
        .map(|date| date.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}
