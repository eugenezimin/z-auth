//! User profile models — labelled phone numbers and postal addresses.
//!
//! Contains **only** plain data structures.
//! - Persistence contract → [`crate::identity::profile::repository`]
//! - Service contract     → [`crate::identity::profile::service`]
//! - Business logic       → [`crate::identity::profile::service_impl`]

use chrono::{DateTime, Utc};
use uuid::Uuid;

// ── Persisted entities ────────────────────────────────────────────────────────

/// A phone number kept by a user.  A user may keep many; at most one is
/// primary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Phone {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Free-text label chosen by the user (`work`, `home`, …).
    pub label: String,
    /// International format (E.164): `+` and 7 to 15 digits.
    pub number: String,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A postal address kept by a user.  A user may keep many; at most one is
/// primary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Free-text label chosen by the user (`work`, `home`, …).
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    /// City, town or village.
    pub locality: Option<String>,
    /// State, province or county.
    pub region: Option<String>,
    pub postal_code: Option<String>,
    /// ISO 3166-1 alpha-2, uppercase (`US`, `DE`).
    pub country_code: String,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Service inputs ────────────────────────────────────────────────────────────

/// A phone number as entered, for adding or replacing one.  The service
/// trims the label and normalizes the number (spaces, `-`, `.`, `(`, `)`
/// are dropped).
///
/// `is_primary = true` makes it the user's primary phone; `false` never
/// demotes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PhoneInput {
    pub label: String,
    pub number: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_primary: bool,
}

/// An address as entered, for adding or replacing one.  Text fields are
/// trimmed; blank optional fields are dropped.
///
/// `is_primary = true` makes it the user's primary address; `false` never
/// demotes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AddressInput {
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_primary: bool,
}

// ── Repository inputs ─────────────────────────────────────────────────────────

/// Validated phone, consumed by
/// [`ProfileRepository`](crate::identity::profile::ProfileRepository).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPhone {
    pub label: String,
    pub number: String,
    /// Make it the user's primary phone, clearing the previous one.
    pub is_primary: bool,
}

/// Validated address, consumed by
/// [`ProfileRepository`](crate::identity::profile::ProfileRepository).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAddress {
    pub label: String,
    pub line1: String,
    pub line2: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: String,
    /// Make it the user's primary address, clearing the previous one.
    pub is_primary: bool,
}
