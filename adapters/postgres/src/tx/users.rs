//! Users: the SQL behind [`PgUserRepository`](crate::PgUserRepository),
//! callable on any connection.

use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::user::{NewUser, User, UserPage, UserQuery, UserUpdate, UserWithRoles};

use crate::errors::map_sqlx_error;
use crate::queries::user_queries as q;
use crate::rows::{UserRow, UserWithRoleRow, fold_user_with_roles};

async fn find_one(
    conn: &mut PgConnection,
    sql: &str,
    value: &str,
) -> Result<Option<User>, AuthError> {
    let row: Option<UserRow> = sqlx::query_as(sql)
        .bind(value)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

async fn find_with_roles<T>(
    conn: &mut PgConnection,
    sql: &str,
    value: T,
) -> Result<Option<UserWithRoles>, AuthError>
where
    T: for<'q> sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send,
{
    let rows: Vec<UserWithRoleRow> = sqlx::query_as(sql)
        .bind(value)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(fold_user_with_roles(rows))
}

async fn exists(conn: &mut PgConnection, sql: &str, value: &str) -> Result<bool, AuthError> {
    let (exists,): (bool,) = sqlx::query_as(sql)
        .bind(value)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(exists)
}

async fn set_flag(conn: &mut PgConnection, sql: &str, user_id: Uuid) -> Result<bool, AuthError> {
    let result = sqlx::query(sql)
        .bind(user_id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

async fn get_flag(
    conn: &mut PgConnection,
    sql: &str,
    user_id: Uuid,
) -> Result<Option<bool>, AuthError> {
    let row: Option<(bool,)> = sqlx::query_as(sql)
        .bind(user_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(|(v,)| v))
}

/// The user with this id.
pub async fn find_by_id(conn: &mut PgConnection, user_id: Uuid) -> Result<Option<User>, AuthError> {
    let row: Option<UserRow> = sqlx::query_as(q::FIND_USER_BY_ID)
        .bind(user_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// The user with this (already normalised) email.
pub async fn find_by_email(
    conn: &mut PgConnection,
    email: &str,
) -> Result<Option<User>, AuthError> {
    find_one(conn, q::FIND_USER_BY_EMAIL, email).await
}

/// The user with this username (case-insensitive).
pub async fn find_by_username(
    conn: &mut PgConnection,
    username: &str,
) -> Result<Option<User>, AuthError> {
    find_one(conn, q::FIND_USER_BY_USERNAME, username).await
}

/// The user with this id and their active roles.
pub async fn find_with_roles_by_id(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Option<UserWithRoles>, AuthError> {
    find_with_roles(conn, q::FIND_USER_WITH_ROLES_BY_ID, user_id).await
}

/// The user with this email and their active roles.
pub async fn find_with_roles_by_email(
    conn: &mut PgConnection,
    email: &str,
) -> Result<Option<UserWithRoles>, AuthError> {
    find_with_roles(conn, q::FIND_USER_WITH_ROLES_BY_EMAIL, email).await
}

/// The user with this username and their active roles.
pub async fn find_with_roles_by_username(
    conn: &mut PgConnection,
    username: &str,
) -> Result<Option<UserWithRoles>, AuthError> {
    find_with_roles(conn, q::FIND_USER_WITH_ROLES_BY_USERNAME, username).await
}

/// One page of users plus the total.  Run it in a transaction when the page
/// and the total must agree.
pub async fn list(conn: &mut PgConnection, query: &UserQuery) -> Result<UserPage, AuthError> {
    let rows: Vec<UserRow> = sqlx::query_as(q::LIST_USERS)
        .bind(&query.search)
        .bind(i64::from(query.limit))
        .bind(i64::from(query.offset))
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let (total,): (i64,) = sqlx::query_as(q::COUNT_USERS)
        .bind(&query.search)
        .fetch_one(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(UserPage {
        items: rows.into_iter().map(Into::into).collect(),
        total: u64::try_from(total).unwrap_or_default(),
    })
}

/// Whether a user has this email.
pub async fn exists_by_email(conn: &mut PgConnection, email: &str) -> Result<bool, AuthError> {
    exists(conn, q::EXISTS_BY_EMAIL, email).await
}

/// Whether a user has this username.
pub async fn exists_by_username(
    conn: &mut PgConnection,
    username: &str,
) -> Result<bool, AuthError> {
    exists(conn, q::EXISTS_BY_USERNAME, username).await
}

/// Insert a user; the database assigns the id.
///
/// Errors: [`AuthError::EmailAlreadyTaken`] / [`AuthError::UsernameAlreadyTaken`].
pub async fn create(conn: &mut PgConnection, new_user: &NewUser) -> Result<User, AuthError> {
    let row: UserRow = sqlx::query_as(q::INSERT_USER)
        .bind(&new_user.email)
        .bind(&new_user.password_hash)
        .bind(&new_user.username)
        .bind(&new_user.first_name)
        .bind(&new_user.last_name)
        .bind(new_user.is_verified)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Insert a user with a caller-chosen id (for hosts that generate ids,
/// e.g. time-ordered UUIDv7).
///
/// Errors: as [`create`], plus [`AuthError::Storage`] on a duplicate id.
pub async fn create_with_id(
    conn: &mut PgConnection,
    user_id: Uuid,
    new_user: &NewUser,
) -> Result<User, AuthError> {
    let row: UserRow = sqlx::query_as(q::INSERT_USER_WITH_ID)
        .bind(user_id)
        .bind(&new_user.email)
        .bind(&new_user.password_hash)
        .bind(&new_user.username)
        .bind(&new_user.first_name)
        .bind(&new_user.last_name)
        .bind(new_user.is_verified)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Partial update: `None` fields are left unchanged.
pub async fn update(
    conn: &mut PgConnection,
    user_id: Uuid,
    update: &UserUpdate,
) -> Result<Option<User>, AuthError> {
    let row: Option<UserRow> = sqlx::query_as(q::UPDATE_USER)
        .bind(&update.email)
        .bind(&update.password_hash)
        .bind(&update.username)
        .bind(&update.first_name)
        .bind(&update.last_name)
        .bind(&update.avatar_url)
        .bind(user_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Delete a user (cascades to their sessions, roles and grants).
pub async fn delete(conn: &mut PgConnection, user_id: Uuid) -> Result<Option<Uuid>, AuthError> {
    let row: Option<(Uuid,)> = sqlx::query_as(q::DELETE_USER)
        .bind(user_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(|(id,)| id))
}

/// Set `is_active = true`.  `false` when there is no such user.
pub async fn activate(conn: &mut PgConnection, user_id: Uuid) -> Result<bool, AuthError> {
    set_flag(conn, q::ACTIVATE_USER, user_id).await
}

/// Set `is_active = false`.  `false` when there is no such user.
pub async fn deactivate(conn: &mut PgConnection, user_id: Uuid) -> Result<bool, AuthError> {
    set_flag(conn, q::DEACTIVATE_USER, user_id).await
}

/// Set `is_verified`.  `false` when there is no such user.
pub async fn set_verified(
    conn: &mut PgConnection,
    user_id: Uuid,
    verified: bool,
) -> Result<bool, AuthError> {
    let result = sqlx::query(q::SET_VERIFIED)
        .bind(user_id)
        .bind(verified)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

/// The user's `is_active` flag; `None` when there is no such user.
pub async fn is_active(conn: &mut PgConnection, user_id: Uuid) -> Result<Option<bool>, AuthError> {
    get_flag(conn, q::GET_IS_ACTIVE, user_id).await
}

/// The user's `is_verified` flag; `None` when there is no such user.
pub async fn is_verified(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Option<bool>, AuthError> {
    get_flag(conn, q::GET_IS_VERIFIED, user_id).await
}
