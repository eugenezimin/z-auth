//! The permission catalog and user / role grants: the SQL behind
//! [`PgPermissionRepository`](crate::PgPermissionRepository), callable on any
//! connection.
//!
//! [`create`], [`replace_user_grant`], [`replace_role_grant`] and
//! [`load_catalog`] run several statements: call them inside a transaction
//! (the repository does).

use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::security::authorization::{
    EffectiveGrants, NewPermission, Permission, PermissionAssignment, PermissionCatalog,
    PermissionGrant, PermissionKind,
};

use crate::enums::PgPermissionKind;
use crate::errors::map_sqlx_error;
use crate::queries::permission_queries as q;
use crate::rows::{GrantRow, OptionRow, PermissionRow};

/// Attach the options to a permission row.
async fn with_options(
    conn: &mut PgConnection,
    row: Option<PermissionRow>,
) -> Result<Option<Permission>, AuthError> {
    let Some(row) = row else {
        return Ok(None);
    };
    let options: Vec<OptionRow> = sqlx::query_as(q::LIST_OPTIONS_OF)
        .bind(row.id())
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.assemble(&options).map(Some)
}

/// The permission with this id, with its options.
pub async fn find_by_id(
    conn: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Permission>, AuthError> {
    let row: Option<PermissionRow> = sqlx::query_as(q::FIND_PERMISSION_BY_ID)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    with_options(conn, row).await
}

async fn require(conn: &mut PgConnection, id: Uuid) -> Result<Permission, AuthError> {
    find_by_id(conn, id)
        .await?
        .ok_or(AuthError::PermissionNotFound)
}

/// An owner's grant rows (ordered by permission), folded into one
/// assignment per permission.
async fn list_grants(
    conn: &mut PgConnection,
    sql: &str,
    owner: Uuid,
) -> Result<Vec<PermissionAssignment>, AuthError> {
    let rows: Vec<(Uuid, Option<Uuid>, Option<String>)> = sqlx::query_as(sql)
        .bind(owner)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    let mut assignments: Vec<PermissionAssignment> = Vec::new();
    for (permission_id, option_id, text) in rows {
        match assignments.last_mut() {
            Some(last) if last.permission_id == permission_id => last.option_ids.extend(option_id),
            _ => assignments.push(PermissionAssignment {
                permission_id,
                option_ids: option_id.into_iter().collect(),
                text,
            }),
        }
    }
    Ok(assignments)
}

/// Delete the owner's grant of a permission, then insert the new rows.
async fn replace_grant(
    conn: &mut PgConnection,
    delete: &str,
    insert: &str,
    owner: Uuid,
    assignment: &PermissionAssignment,
) -> Result<(), AuthError> {
    sqlx::query(delete)
        .bind(owner)
        .bind(assignment.permission_id)
        .execute(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let option_ids: Vec<Option<Uuid>> = if assignment.option_ids.is_empty() {
        vec![None]
    } else {
        assignment.option_ids.iter().copied().map(Some).collect()
    };
    for option_id in option_ids {
        sqlx::query(insert)
            .bind(owner)
            .bind(assignment.permission_id)
            .bind(option_id)
            .bind(&assignment.text)
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx_error)?;
    }
    Ok(())
}

async fn delete_grant(
    conn: &mut PgConnection,
    sql: &str,
    owner: Uuid,
    permission_id: Uuid,
) -> Result<bool, AuthError> {
    let result = sqlx::query(sql)
        .bind(owner)
        .bind(permission_id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

/// Define a permission with its options.  Several statements: run it in a
/// transaction.
///
/// Errors: [`AuthError::PermissionAlreadyExists`] on a duplicate code.
pub async fn create(
    conn: &mut PgConnection,
    permission: &NewPermission,
) -> Result<Permission, AuthError> {
    let (has_position, max_length) =
        match permission.kind {
            PermissionKind::Bool => (true, None),
            PermissionKind::Text { max_length } => (
                true,
                Some(i32::try_from(max_length).map_err(|_| {
                    AuthError::InvalidPermissionValue("max_length too large".into())
                })?),
            ),
            PermissionKind::Single | PermissionKind::Multi => (false, None),
        };
    let (id,): (Uuid,) = sqlx::query_as(q::INSERT_PERMISSION)
        .bind(&permission.code)
        .bind(PgPermissionKind::from(permission.kind))
        .bind(&permission.description)
        .bind(has_position)
        .bind(max_length)
        .fetch_one(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    for option in &permission.options {
        sqlx::query(q::INSERT_OPTION)
            .bind(id)
            .bind(option)
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx_error)?;
    }
    require(conn, id).await
}

/// Add an option to a choice permission.
pub async fn add_option(
    conn: &mut PgConnection,
    permission_id: Uuid,
    option_code: &str,
) -> Result<Permission, AuthError> {
    sqlx::query(q::INSERT_OPTION)
        .bind(permission_id)
        .bind(option_code)
        .execute(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    require(conn, permission_id).await
}

/// Delete a permission (cascades to its options and grants).
pub async fn delete(conn: &mut PgConnection, permission_id: Uuid) -> Result<bool, AuthError> {
    let result = sqlx::query(q::DELETE_PERMISSION)
        .bind(permission_id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

/// The permission with this code, with its options.
pub async fn find_by_code(
    conn: &mut PgConnection,
    code: &str,
) -> Result<Option<Permission>, AuthError> {
    let row: Option<PermissionRow> = sqlx::query_as(q::FIND_PERMISSION_BY_CODE)
        .bind(code)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    with_options(conn, row).await
}

/// The whole catalog.  Several statements: run it in a transaction for a
/// consistent read.
pub async fn load_catalog(conn: &mut PgConnection) -> Result<PermissionCatalog, AuthError> {
    let rows: Vec<PermissionRow> = sqlx::query_as(q::LIST_PERMISSIONS)
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let options: Vec<OptionRow> = sqlx::query_as(q::LIST_OPTIONS)
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let version = catalog_version(conn).await?;
    Ok(PermissionCatalog {
        version,
        permissions: rows
            .into_iter()
            .map(|r| r.assemble(&options))
            .collect::<Result<_, _>>()?,
    })
}

async fn catalog_version(conn: &mut PgConnection) -> Result<u64, AuthError> {
    let (version,): (i64,) = sqlx::query_as(q::CATALOG_VERSION)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(u64::try_from(version).unwrap_or_default())
}

/// Replace the user's grant of one permission.  Several statements: run it
/// in a transaction.
pub async fn replace_user_grant(
    conn: &mut PgConnection,
    user_id: Uuid,
    assignment: &PermissionAssignment,
) -> Result<(), AuthError> {
    replace_grant(
        conn,
        q::DELETE_USER_GRANT,
        q::INSERT_USER_GRANT,
        user_id,
        assignment,
    )
    .await
}

/// Remove the user's grant of one permission.  `false` when there was none.
pub async fn clear_user_grant(
    conn: &mut PgConnection,
    user_id: Uuid,
    permission_id: Uuid,
) -> Result<bool, AuthError> {
    delete_grant(conn, q::DELETE_USER_GRANT, user_id, permission_id).await
}

/// Replace the role's grant of one permission.  Several statements: run it
/// in a transaction.
pub async fn replace_role_grant(
    conn: &mut PgConnection,
    role_id: Uuid,
    assignment: &PermissionAssignment,
) -> Result<(), AuthError> {
    replace_grant(
        conn,
        q::DELETE_ROLE_GRANT,
        q::INSERT_ROLE_GRANT,
        role_id,
        assignment,
    )
    .await
}

/// Remove the role's grant of one permission.  `false` when there was none.
pub async fn clear_role_grant(
    conn: &mut PgConnection,
    role_id: Uuid,
    permission_id: Uuid,
) -> Result<bool, AuthError> {
    delete_grant(conn, q::DELETE_ROLE_GRANT, role_id, permission_id).await
}

/// The user's direct grants, one assignment per permission.
pub async fn list_user_grants(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<PermissionAssignment>, AuthError> {
    list_grants(conn, q::LIST_USER_GRANTS, user_id).await
}

/// The role's grants, one assignment per permission.
pub async fn list_role_grants(
    conn: &mut PgConnection,
    role_id: Uuid,
) -> Result<Vec<PermissionAssignment>, AuthError> {
    list_grants(conn, q::LIST_ROLE_GRANTS, role_id).await
}

/// The grants a user's token carries: direct ones, plus those of their
/// active roles when `include_role_grants`.
pub async fn effective_grants(
    conn: &mut PgConnection,
    user_id: Uuid,
    include_role_grants: bool,
) -> Result<EffectiveGrants, AuthError> {
    let rows: Vec<GrantRow> = sqlx::query_as(q::EFFECTIVE_GRANTS)
        .bind(user_id)
        .bind(include_role_grants)
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let catalog_version = catalog_version(conn).await?;
    Ok(EffectiveGrants {
        catalog_version,
        grants: rows
            .into_iter()
            .map(PermissionGrant::try_from)
            .collect::<Result<_, _>>()?,
    })
}
