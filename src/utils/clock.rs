//! Local time source for **stateless** checks only: access-token `exp`
//! (with leeway) and expiry of in-memory denylist entries.
//!
//! Everything persisted — session and generation timestamps, expiries,
//! revocation cutoffs — is stamped by the store's own clock, which is the
//! source of truth; see [`SessionRepository`](crate::security::session::SessionRepository).

use time::OffsetDateTime;

pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

/// The system wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}
