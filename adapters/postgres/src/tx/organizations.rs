//! Organizations, departments and memberships: the SQL behind
//! [`PgOrganizationRepository`](crate::PgOrganizationRepository), callable
//! on any connection (feature `profile`).
//!
//! The membership writes that may change a user's primary membership
//! ([`add_membership`], [`update_membership`], [`set_primary_membership`])
//! run several statements: call them inside a transaction (the repository
//! does).  Each first locks the user's row, so concurrent primary changes
//! for one user wait for each other until the transaction ends.  A
//! membership that is not found changes nothing.  Memberships are read
//! with their organization's and department's current names.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::organization::{
    Department, DepartmentInput, DepartmentRef, MemberQuery, Membership, MembershipPage,
    NewMembership, Organization, OrganizationInput, OrganizationPage, OrganizationQuery,
    OrganizationRef,
};

use super::profiles::{clear_primary, delete_item, item_exists, lock_user};
use crate::errors::map_sqlx_error;
use crate::profile::organization_queries as q;

#[derive(sqlx::FromRow)]
struct OrganizationRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrganizationRow> for Organization {
    fn from(r: OrganizationRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DepartmentRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DepartmentRow> for Department {
    fn from(r: DepartmentRow) -> Self {
        Self {
            id: r.id,
            organization_id: r.organization_id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// A membership joined with its organization's and department's names.
#[derive(sqlx::FromRow)]
struct MembershipRow {
    id: Uuid,
    user_id: Uuid,
    organization_id: Uuid,
    organization_name: String,
    department_id: Option<Uuid>,
    department_name: Option<String>,
    title: Option<String>,
    is_primary: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<MembershipRow> for Membership {
    fn from(r: MembershipRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            organization: OrganizationRef {
                id: r.organization_id,
                name: r.organization_name,
            },
            department: r
                .department_id
                .zip(r.department_name)
                .map(|(id, name)| DepartmentRef { id, name }),
            title: r.title,
            is_primary: r.is_primary,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Organizations ─────────────────────────────────────────────────────────────

/// The organization with this id.
pub async fn find_organization(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Organization>, AuthError> {
    let row: Option<OrganizationRow> = sqlx::query_as(q::FIND_ORGANIZATION)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// One page of organizations by name plus the total.  Run it in a
/// transaction when the page and the total must agree.
pub async fn list_organizations(
    conn: &mut PgConnection,
    query: &OrganizationQuery,
) -> Result<OrganizationPage, AuthError> {
    let rows: Vec<OrganizationRow> = sqlx::query_as(q::LIST_ORGANIZATIONS)
        .bind(&query.search)
        .bind(i64::from(query.limit))
        .bind(i64::from(query.offset))
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let (total,): (i64,) = sqlx::query_as(q::COUNT_ORGANIZATIONS)
        .bind(&query.search)
        .fetch_one(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(OrganizationPage {
        items: rows.into_iter().map(Into::into).collect(),
        total: u64::try_from(total).unwrap_or_default(),
    })
}

/// Insert an organization.
///
/// Errors: [`AuthError::OrganizationAlreadyExists`].
pub async fn create_organization(
    conn: &mut PgConnection,
    input: &OrganizationInput,
) -> Result<Organization, AuthError> {
    let row: OrganizationRow = sqlx::query_as(q::INSERT_ORGANIZATION)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Replace an organization's name and description.  Errors: as
/// [`create_organization`].
pub async fn update_organization(
    conn: &mut PgConnection,
    id: Uuid,
    input: &OrganizationInput,
) -> Result<Option<Organization>, AuthError> {
    let row: Option<OrganizationRow> = sqlx::query_as(q::UPDATE_ORGANIZATION)
        .bind(id)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Delete an organization (cascades to its departments and memberships).
pub async fn delete_organization(conn: &mut PgConnection, id: Uuid) -> Result<bool, AuthError> {
    let result = sqlx::query(q::DELETE_ORGANIZATION)
        .bind(id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

// ── Departments ───────────────────────────────────────────────────────────────

/// An organization's departments by name.
pub async fn list_departments(
    conn: &mut PgConnection,
    organization_id: Uuid,
) -> Result<Vec<Department>, AuthError> {
    let rows: Vec<DepartmentRow> = sqlx::query_as(q::LIST_DEPARTMENTS)
        .bind(organization_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// The department with this id.
pub async fn find_department(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Department>, AuthError> {
    let row: Option<DepartmentRow> = sqlx::query_as(q::FIND_DEPARTMENT)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Insert a department.
///
/// Errors: [`AuthError::OrganizationNotFound`],
/// [`AuthError::DepartmentAlreadyExists`].
pub async fn create_department(
    conn: &mut PgConnection,
    organization_id: Uuid,
    input: &DepartmentInput,
) -> Result<Department, AuthError> {
    let row: DepartmentRow = sqlx::query_as(q::INSERT_DEPARTMENT)
        .bind(organization_id)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Replace a department's name and description.
///
/// Errors: [`AuthError::DepartmentAlreadyExists`].
pub async fn update_department(
    conn: &mut PgConnection,
    id: Uuid,
    input: &DepartmentInput,
) -> Result<Option<Department>, AuthError> {
    let row: Option<DepartmentRow> = sqlx::query_as(q::UPDATE_DEPARTMENT)
        .bind(id)
        .bind(&input.name)
        .bind(&input.description)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Delete a department; its memberships stay, with no department.
pub async fn delete_department(conn: &mut PgConnection, id: Uuid) -> Result<bool, AuthError> {
    let result = sqlx::query(q::DELETE_DEPARTMENT)
        .bind(id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

// ── Memberships ───────────────────────────────────────────────────────────────

/// A user's memberships, the primary first, then oldest first.
pub async fn list_memberships(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<Membership>, AuthError> {
    let rows: Vec<MembershipRow> = sqlx::query_as(q::LIST_MEMBERSHIPS)
        .bind(user_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// One page of an organization's memberships, oldest first, plus the
/// total.  Run it in a transaction when the page and the total must agree.
pub async fn list_members(
    conn: &mut PgConnection,
    organization_id: Uuid,
    query: &MemberQuery,
) -> Result<MembershipPage, AuthError> {
    let rows: Vec<MembershipRow> = sqlx::query_as(q::LIST_MEMBERS)
        .bind(organization_id)
        .bind(query.department_id)
        .bind(i64::from(query.limit))
        .bind(i64::from(query.offset))
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let (total,): (i64,) = sqlx::query_as(q::COUNT_MEMBERS)
        .bind(organization_id)
        .bind(query.department_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(MembershipPage {
        items: rows.into_iter().map(Into::into).collect(),
        total: u64::try_from(total).unwrap_or_default(),
    })
}

/// Insert a membership, clearing the previous primary when
/// `membership.is_primary`.  Several statements: run it in a transaction.
///
/// Errors: [`AuthError::UserNotFound`], [`AuthError::OrganizationNotFound`],
/// [`AuthError::DepartmentNotFound`] (also for a department of another
/// organization), [`AuthError::MembershipAlreadyExists`].
pub async fn add_membership(
    conn: &mut PgConnection,
    user_id: Uuid,
    membership: &NewMembership,
) -> Result<Membership, AuthError> {
    if !lock_user(conn, user_id).await? {
        return Err(AuthError::UserNotFound);
    }
    if membership.is_primary {
        clear_primary(conn, q::CLEAR_PRIMARY_MEMBERSHIP, user_id, None).await?;
    }
    let row: MembershipRow = sqlx::query_as(q::INSERT_MEMBERSHIP)
        .bind(user_id)
        .bind(membership.organization_id)
        .bind(membership.department_id)
        .bind(&membership.title)
        .bind(membership.is_primary)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Replace a membership's organization, department and title;
/// `membership.is_primary` makes it the primary one, `false` keeps its
/// flag.  `Ok(None)`, changing nothing, if the user has no such membership.
/// Several statements: run it in a transaction.  Errors: as
/// [`add_membership`], except `UserNotFound`.
pub async fn update_membership(
    conn: &mut PgConnection,
    user_id: Uuid,
    membership_id: Uuid,
    membership: &NewMembership,
) -> Result<Option<Membership>, AuthError> {
    if !lock_user(conn, user_id).await? {
        return Ok(None);
    }
    let row: Option<MembershipRow> = sqlx::query_as(q::UPDATE_MEMBERSHIP)
        .bind(membership_id)
        .bind(user_id)
        .bind(membership.organization_id)
        .bind(membership.department_id)
        .bind(&membership.title)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    match row {
        Some(row) if membership.is_primary && !row.is_primary => {
            mark_primary_membership(conn, user_id, membership_id)
                .await
                .map(Some)
        }
        row => Ok(row.map(Into::into)),
    }
}

/// Make a membership the user's primary one.  `Ok(false)`, changing
/// nothing, if the user has no such membership.  Several statements: run
/// it in a transaction.
pub async fn set_primary_membership(
    conn: &mut PgConnection,
    user_id: Uuid,
    membership_id: Uuid,
) -> Result<bool, AuthError> {
    if !lock_user(conn, user_id).await?
        || !item_exists(conn, q::MEMBERSHIP_EXISTS, membership_id, user_id).await?
    {
        return Ok(false);
    }
    mark_primary_membership(conn, user_id, membership_id).await?;
    Ok(true)
}

/// Clear the previous primary, then flag the existing membership
/// `membership_id`, and read it back with its names.
async fn mark_primary_membership(
    conn: &mut PgConnection,
    user_id: Uuid,
    membership_id: Uuid,
) -> Result<Membership, AuthError> {
    clear_primary(
        conn,
        q::CLEAR_PRIMARY_MEMBERSHIP,
        user_id,
        Some(membership_id),
    )
    .await?;
    sqlx::query(q::SET_PRIMARY_MEMBERSHIP)
        .bind(membership_id)
        .bind(user_id)
        .execute(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let row: MembershipRow = sqlx::query_as(q::FIND_MEMBERSHIP)
        .bind(membership_id)
        .bind(user_id)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Delete a membership.  `Ok(false)` if the user has no such membership.
pub async fn delete_membership(
    conn: &mut PgConnection,
    user_id: Uuid,
    membership_id: Uuid,
) -> Result<bool, AuthError> {
    delete_item(conn, q::DELETE_MEMBERSHIP, membership_id, user_id).await
}
