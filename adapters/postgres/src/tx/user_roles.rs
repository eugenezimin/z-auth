//! Role assignments: the SQL behind
//! [`PgUserRoleRepository`](crate::PgUserRoleRepository), callable on any
//! connection.

use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::access::UserRole;

use crate::errors::map_sqlx_error;
use crate::queries::user_role_queries as q;
use crate::rows::UserRoleRow;

async fn list(
    conn: &mut PgConnection,
    sql: &str,
    user_id: Uuid,
) -> Result<Vec<UserRole>, AuthError> {
    let rows: Vec<UserRoleRow> = sqlx::query_as(sql)
        .bind(user_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Assign a role.  `false` when the user already holds it actively.
pub async fn assign(
    conn: &mut PgConnection,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<bool, AuthError> {
    // `ON CONFLICT DO NOTHING` → no row returned when the user already
    // holds the role actively (partial unique index).
    let row: Option<UserRoleRow> = sqlx::query_as(q::INSERT_USER_ROLE)
        .bind(user_id)
        .bind(role_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.is_some())
}

/// Revoke an active assignment (kept as history).  `false` when there was
/// none.
pub async fn revoke(
    conn: &mut PgConnection,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<bool, AuthError> {
    let row: Option<UserRoleRow> = sqlx::query_as(q::REVOKE_USER_ROLE)
        .bind(user_id)
        .bind(role_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.is_some())
}

/// Whether the user holds the role actively.
pub async fn is_role_active(
    conn: &mut PgConnection,
    user_id: Uuid,
    role_id: Uuid,
) -> Result<bool, AuthError> {
    let (exists,): (bool,) = sqlx::query_as(q::IS_ROLE_ACTIVE)
        .bind(user_id)
        .bind(role_id)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(exists)
}

/// The user's active assignments.
pub async fn list_active_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<UserRole>, AuthError> {
    list(conn, q::LIST_ACTIVE_FOR_USER, user_id).await
}

/// Every assignment the user ever had, revoked ones included.
pub async fn list_all_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<UserRole>, AuthError> {
    list(conn, q::LIST_ALL_FOR_USER, user_id).await
}

/// Revoke every active assignment of the user; returns how many.
pub async fn revoke_all_for_user(conn: &mut PgConnection, user_id: Uuid) -> Result<u64, AuthError> {
    let result = sqlx::query(q::REVOKE_ALL_FOR_USER)
        .bind(user_id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected())
}

/// The users holding the role actively.
pub async fn list_active_holders(
    conn: &mut PgConnection,
    role_id: Uuid,
) -> Result<Vec<Uuid>, AuthError> {
    sqlx::query_scalar(q::LIST_ACTIVE_HOLDERS)
        .bind(role_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)
}
