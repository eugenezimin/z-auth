//! The JSON form of a second factor as the API shows it: timestamps in
//! RFC 3339 UTC with `Z` and 0, 3, 6 or 9 fractional digits, unchanged when
//! the time library behind them changes.

use uuid::Uuid;
use z_auth_mfa::model::{FactorView, MfaMethod};

/// 2026-11-18T11:06:40Z.
const T: i64 = 1_795_000_000;

const FACTOR: &str = r#"{"id":"00000000-0000-0000-0000-000000000001","method":"totp","label":"phone","destination_hint":null,"created_at":"2026-11-18T11:06:40.001Z","confirmed_at":"2026-11-18T11:06:45Z","last_used_at":null}"#;

/// The one place this test builds a timestamp.
fn at(secs: i64, nanos: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, nanos).unwrap()
}

#[test]
fn a_factor_keeps_its_json() {
    let factor = FactorView {
        id: Uuid::from_u128(1),
        method: MfaMethod::Totp,
        label: Some("phone".into()),
        destination_hint: None,
        created_at: at(T, 1_000_000),
        confirmed_at: Some(at(T + 5, 0)),
        last_used_at: None,
    };
    let json = serde_json::to_string(&factor).unwrap();
    assert_eq!(json, FACTOR);
}
