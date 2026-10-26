//! Roles: the SQL behind [`PgRoleRepository`](crate::PgRoleRepository),
//! callable on any connection.

use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::access::{NewRole, Role, RoleUpdate};

use crate::errors::map_sqlx_error;
use crate::queries::role_queries as q;
use crate::rows::RoleRow;

async fn find_one<T>(
    conn: &mut PgConnection,
    sql: &str,
    value: T,
) -> Result<Option<Role>, AuthError>
where
    T: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
{
    let row: Option<RoleRow> = sqlx::query_as(sql)
        .bind(value)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Insert a role; the database assigns the id.
///
/// Errors: [`AuthError::RoleAlreadyExists`] on a duplicate code or name.
pub async fn create(conn: &mut PgConnection, new_role: &NewRole) -> Result<Role, AuthError> {
    let row: RoleRow = sqlx::query_as(q::INSERT_ROLE)
        .bind(&new_role.code)
        .bind(&new_role.name)
        .bind(&new_role.description)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Insert a role with a caller-chosen id.  Errors: as [`create`].
pub async fn create_with_id(
    conn: &mut PgConnection,
    role_id: Uuid,
    new_role: &NewRole,
) -> Result<Role, AuthError> {
    let row: RoleRow = sqlx::query_as(q::INSERT_ROLE_WITH_ID)
        .bind(role_id)
        .bind(&new_role.code)
        .bind(&new_role.name)
        .bind(&new_role.description)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// The role with this id.
pub async fn find_by_id(conn: &mut PgConnection, id: Uuid) -> Result<Option<Role>, AuthError> {
    find_one(conn, q::FIND_ROLE_BY_ID, id).await
}

/// The role with this display name.
pub async fn find_by_name(conn: &mut PgConnection, name: &str) -> Result<Option<Role>, AuthError> {
    find_one(conn, q::FIND_ROLE_BY_NAME, name).await
}

/// The role with this code.
pub async fn find_by_code(conn: &mut PgConnection, code: &str) -> Result<Option<Role>, AuthError> {
    find_one(conn, q::FIND_ROLE_BY_CODE, code).await
}

/// Every role.
pub async fn list_all(conn: &mut PgConnection) -> Result<Vec<Role>, AuthError> {
    let rows: Vec<RoleRow> = sqlx::query_as(q::LIST_ALL_ROLES)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Update the name and/or description; the code never changes.
pub async fn update(
    conn: &mut PgConnection,
    id: Uuid,
    update: &RoleUpdate,
) -> Result<Option<Role>, AuthError> {
    let row: Option<RoleRow> = sqlx::query_as(q::UPDATE_ROLE)
        .bind(&update.name)
        .bind(update.description.is_some())
        .bind(update.description.clone().flatten())
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Delete a role (cascades to its assignments and grants).
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> Result<Option<Uuid>, AuthError> {
    let row: Option<(Uuid,)> = sqlx::query_as(q::DELETE_ROLE)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(|(id,)| id))
}

/// Whether a role has this display name.
pub async fn exists_by_name(conn: &mut PgConnection, name: &str) -> Result<bool, AuthError> {
    let (exists,): (bool,) = sqlx::query_as(q::EXISTS_BY_NAME)
        .bind(name)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(exists)
}
