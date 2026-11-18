//! The JSON form of the sessions the cluster syncs between nodes: timestamps
//! in RFC 3339 UTC with `Z` and 0, 3, 6 or 9 fractional digits. Nodes of
//! different versions exchange it, so it must not change when the time
//! library behind it does.

use std::net::{IpAddr, Ipv4Addr};

use uuid::Uuid;
use z_auth::security::session::model::{RevocationReason, SessionStatus};
use z_auth_cluster::sync::SyncSession;

/// 2026-11-18T11:06:40Z.
const T: i64 = 1_795_000_000;

const SESSION: &str = r#"{"id":"00000000-0000-0000-0000-000000000001","user_id":"00000000-0000-0000-0000-000000000002","secret":[1,2,3],"status":"revoked","end_reason":"account_changed","created_ip":"10.0.0.1","user_agent":null,"amr":["pwd","otp"],"current_generation":3,"created_at":"2026-11-18T11:06:40Z","idle_expires_at":"2026-11-18T11:36:40.500Z","absolute_expires_at":"2026-11-18T19:06:40.123456Z","ended_at":"2026-11-18T11:07:40.123456789Z"}"#;

/// The one place this test builds a timestamp.
fn at(secs: i64, nanos: u32) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(secs)
        .unwrap()
        .replace_nanosecond(nanos)
        .unwrap()
}

#[test]
fn a_synced_session_keeps_its_json() {
    let session = SyncSession {
        id: Uuid::from_u128(1),
        user_id: Uuid::from_u128(2),
        secret: vec![1, 2, 3],
        status: SessionStatus::Revoked,
        end_reason: Some(RevocationReason::AccountChanged),
        created_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        user_agent: None,
        amr: vec!["pwd".into(), "otp".into()],
        current_generation: 3,
        created_at: at(T, 0),
        idle_expires_at: at(T + 1_800, 500_000_000),
        absolute_expires_at: at(T + 28_800, 123_456_000),
        ended_at: Some(at(T + 60, 123_456_789)),
    };
    let json = serde_json::to_string(&session).unwrap();
    assert_eq!(json, SESSION);
    let back: SyncSession = serde_json::from_str(&json).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}
