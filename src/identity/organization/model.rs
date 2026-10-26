//! Organization models — a shared catalog of organizations and their
//! departments, and users' memberships in them.
//!
//! Contains **only** plain data structures.
//! - Persistence contract → [`crate::identity::organization::repository`]
//! - Service contract     → [`crate::identity::organization::service`]
//! - Business logic       → [`crate::identity::organization::service_impl`]

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Page size used when a query asks for `limit = 0`.
pub const DEFAULT_ORGANIZATION_PAGE: u32 = 50;
/// Largest page of organizations or members returned.
pub const MAX_ORGANIZATION_PAGE: u32 = 200;

// ── Catalog ───────────────────────────────────────────────────────────────────

/// An organization users may belong to.  Names are unique,
/// case-insensitively.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A department of an organization.  Names are unique within the
/// organization, case-insensitively.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Department {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An organization to create, or every field of one to replace.  The
/// service trims both fields; a blank description is dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrganizationInput {
    pub name: String,
    pub description: Option<String>,
}

/// A department to create, or every field of one to replace.  Trimmed as
/// [`OrganizationInput`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DepartmentInput {
    pub name: String,
    pub description: Option<String>,
}

/// Page request for organizations.  `search`, when set, is matched
/// case-insensitively as a substring of the name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrganizationQuery {
    pub search: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

/// One page of organizations ordered by name, plus the total matching
/// `search`.
#[derive(Debug, Clone)]
pub struct OrganizationPage {
    pub items: Vec<Organization>,
    pub total: u64,
}

// ── Memberships ───────────────────────────────────────────────────────────────

/// The organization of a [`Membership`], by id and current name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrganizationRef {
    pub id: Uuid,
    pub name: String,
}

/// The department of a [`Membership`], by id and current name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DepartmentRef {
    pub id: Uuid,
    pub name: String,
}

/// A user's membership in an organization, optionally in one of its
/// departments.  A user may hold many; at most one is primary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Membership {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization: OrganizationRef,
    /// `None` when the user belongs to the organization as a whole, or the
    /// department was deleted.
    pub department: Option<DepartmentRef>,
    /// Job title within the organization.
    pub title: Option<String>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A membership as entered, for adding or replacing one.  `department_id`
/// must be a department of `organization_id`.
///
/// `is_primary = true` makes it the user's primary membership; `false`
/// never demotes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MembershipInput {
    pub organization_id: Uuid,
    pub department_id: Option<Uuid>,
    pub title: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_primary: bool,
}

/// Validated membership, consumed by
/// [`OrganizationRepository`](crate::identity::organization::OrganizationRepository).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMembership {
    pub organization_id: Uuid,
    pub department_id: Option<Uuid>,
    pub title: Option<String>,
    /// Make it the user's primary membership, clearing the previous one.
    pub is_primary: bool,
}

/// Page request for an organization's members.  `department_id`, when set,
/// keeps only that department's members.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemberQuery {
    pub department_id: Option<Uuid>,
    pub limit: u32,
    pub offset: u32,
}

/// One page of memberships, oldest first, plus the total matching the
/// query.
#[derive(Debug, Clone)]
pub struct MembershipPage {
    pub items: Vec<Membership>,
    pub total: u64,
}
