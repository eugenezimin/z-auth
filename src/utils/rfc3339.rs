//! Timestamps in serialized models, written exactly as z-auth has always
//! written them: RFC 3339 in UTC with `Z` and 0, 3, 6 or 9 fractional
//! digits, whichever is exact (`2026-11-18T11:06:40Z`, `…40.500Z`,
//! `…40.123456Z`, `…40.123456789Z`). Hosts sync these models between
//! services and the cluster signs envelopes over them, so the text stays the
//! same whatever the time library. Any RFC 3339 timestamp is read and moved
//! to UTC.
//!
//! [`format()`] and [`parse()`] are always available; the serde functions
//! need the `serde` feature. Use them on a field as `#[serde(with = "z_auth::utils::rfc3339")]`, or
//! `rfc3339::option` with `#[serde(default)]` for an `Option`.

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serializer};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// The text of `at` in UTC, as described in the [module](self).
#[must_use]
pub fn format(at: OffsetDateTime) -> String {
    let at = at.to_offset(UtcOffset::UTC);
    let nanos = at.nanosecond();
    let fraction = if nanos == 0 {
        String::new()
    } else if nanos.is_multiple_of(1_000_000) {
        format!(".{:03}", nanos / 1_000_000)
    } else if nanos.is_multiple_of(1_000) {
        format!(".{:06}", nanos / 1_000)
    } else {
        format!(".{nanos:09}")
    };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{fraction}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// Read an RFC 3339 timestamp and move it to UTC.
///
/// # Errors
/// [`time::error::Parse`] when the text is not RFC 3339.
pub fn parse(text: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(text, &Rfc3339).map(|at| at.to_offset(UtcOffset::UTC))
}

#[cfg(feature = "serde")]
/// Serialize as [`format()`] writes it.
///
/// # Errors
/// Whatever the serializer returns.
pub fn serialize<S: Serializer>(at: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&format(*at))
}

#[cfg(feature = "serde")]
/// Deserialize any RFC 3339 timestamp, in UTC.
///
/// # Errors
/// A custom error when the text is not RFC 3339.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OffsetDateTime, D::Error> {
    let text = String::deserialize(deserializer)?;
    parse(&text).map_err(serde::de::Error::custom)
}

#[cfg(feature = "serde")]
/// The same for an `Option<OffsetDateTime>`; pair it with `#[serde(default)]`
/// so that a missing field reads as `None`.
pub mod option {
    use super::{Deserialize, Deserializer, OffsetDateTime, Serializer, format, parse};

    /// Serialize `Some` as [`format()`](super::format) writes it, `None` as null.
    ///
    /// # Errors
    /// Whatever the serializer returns.
    pub fn serialize<S: Serializer>(
        at: &Option<OffsetDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match at {
            Some(at) => serializer.serialize_some(&format(*at)),
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize null or any RFC 3339 timestamp, in UTC.
    ///
    /// # Errors
    /// A custom error when the text is not RFC 3339.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<OffsetDateTime>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|text| parse(&text).map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64, nanos: u32) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(secs)
            .map(|t| t.replace_nanosecond(nanos).unwrap_or(t))
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
    }

    #[test]
    fn writes_zero_three_six_or_nine_digits() {
        let t = 1_795_000_000;
        for (nanos, text) in [
            (0, "2026-11-18T11:06:40Z"),
            (500_000_000, "2026-11-18T11:06:40.500Z"),
            (1_000_000, "2026-11-18T11:06:40.001Z"),
            (123_456_000, "2026-11-18T11:06:40.123456Z"),
            (123_456_789, "2026-11-18T11:06:40.123456789Z"),
            (100, "2026-11-18T11:06:40.000000100Z"),
        ] {
            assert_eq!(format(at(t, nanos)), text);
            assert_eq!(parse(text).ok(), Some(at(t, nanos)), "{text}");
        }
    }

    #[test]
    fn reads_other_offsets_into_utc() {
        assert_eq!(
            parse("2026-11-18T14:06:40.5+03:00")
                .map(format)
                .ok()
                .as_deref(),
            Some("2026-11-18T11:06:40.500Z")
        );
        assert!(parse("2026-11-18 11:06:40").is_err());
    }
}
