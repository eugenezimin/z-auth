//! The JSON form of timestamps in z-auth's serializable models: RFC 3339 in
//! UTC with `Z`, and 0, 3, 6 or 9 fractional digits, whichever is exact.
//! Hosts sync these models between services and sign cluster envelopes over
//! them, so the text must not change when the time library behind them does.
#![cfg(feature = "serde")]

use uuid::Uuid;
use z_auth::identity::organization::model::Organization;
use z_auth::security::session::model::RevocationReason;
use z_auth::security::token::model::{Revocation, RevocationScope, RevocationStatus};

/// 2026-11-18T11:06:40Z.
const T: i64 = 1_795_000_000;

/// The one place these tests build a timestamp.
fn at(secs: i64, nanos: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(secs, nanos).unwrap()
}

const REVOCATION: &str = r#"{"id":"00000000-0000-0000-0000-000000000001","scope":{"type":"session","id":"00000000-0000-0000-0000-000000000002"},"reason":"account_changed","status":"pending","origin_node":"00000000-0000-0000-0000-000000000003","revoked_at":"2026-11-18T11:06:40Z","expires_at":"2026-11-18T11:11:40.500Z","enforced_at":"2026-11-18T11:06:41.123456789Z","enforced_by":null}"#;
const ORGANIZATION: &str = r#"{"id":"00000000-0000-0000-0000-000000000004","name":"Acme","description":null,"created_at":"2026-11-18T11:06:40.123456Z","updated_at":"2026-11-19T11:06:40.001Z"}"#;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

#[test]
fn revocation_keeps_its_json() {
    let revocation = Revocation {
        id: id(1),
        scope: RevocationScope::Session(id(2)),
        reason: RevocationReason::AccountChanged,
        status: RevocationStatus::Pending,
        origin_node: id(3),
        revoked_at: at(T, 0),
        expires_at: at(T + 300, 500_000_000),
        enforced_at: Some(at(T + 1, 123_456_789)),
        enforced_by: None,
    };
    let json = serde_json::to_string(&revocation).unwrap();
    assert_eq!(json, REVOCATION);
    let back: Revocation = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn organization_keeps_its_json() {
    let organization = Organization {
        id: id(4),
        name: "Acme".into(),
        description: None,
        created_at: at(T, 123_456_000),
        updated_at: at(T + 86_400, 1_000_000),
    };
    let json = serde_json::to_string(&organization).unwrap();
    assert_eq!(json, ORGANIZATION);
    let back: Organization = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}
