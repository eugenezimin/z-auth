//! Phones and addresses: the SQL behind
//! [`PgProfileRepository`](crate::PgProfileRepository), callable on any
//! connection (feature `profile`).
//!
//! The writes that may change a user's primary item ([`add_phone`],
//! [`replace_phone`], [`set_primary_phone`] and their address twins) run
//! several statements: call them inside a transaction (the repository
//! does).  Each first locks the user's row, so concurrent primary changes
//! for one user wait for each other until the transaction ends.  An item
//! that is not found changes nothing.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::profile::{Address, NewAddress, NewPhone, Phone};

use crate::errors::map_sqlx_error;
use crate::profile::profile_queries as q;

#[derive(sqlx::FromRow)]
struct PhoneRow {
    id: Uuid,
    user_id: Uuid,
    label: String,
    number: String,
    is_primary: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PhoneRow> for Phone {
    fn from(r: PhoneRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            label: r.label,
            number: r.number,
            is_primary: r.is_primary,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AddressRow {
    id: Uuid,
    user_id: Uuid,
    label: String,
    line1: String,
    line2: Option<String>,
    locality: Option<String>,
    region: Option<String>,
    postal_code: Option<String>,
    country_code: String,
    is_primary: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AddressRow> for Address {
    fn from(r: AddressRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            label: r.label,
            line1: r.line1,
            line2: r.line2,
            locality: r.locality,
            region: r.region,
            postal_code: r.postal_code,
            country_code: r.country_code,
            is_primary: r.is_primary,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ── Shared by phones, addresses and memberships ───────────────────────────────

/// Lock the user's row until the transaction ends.  `false`: no such user.
pub(crate) async fn lock_user(conn: &mut PgConnection, user_id: Uuid) -> Result<bool, AuthError> {
    let row: Option<(Uuid,)> = sqlx::query_as(q::LOCK_USER)
        .bind(user_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.is_some())
}

/// Run a `CLEAR_PRIMARY_*` statement: clear the user's primary item, except
/// `except`.
pub(crate) async fn clear_primary(
    conn: &mut PgConnection,
    sql: &str,
    user_id: Uuid,
    except: Option<Uuid>,
) -> Result<(), AuthError> {
    sqlx::query(sql)
        .bind(user_id)
        .bind(except)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(())
}

/// Run an `*_EXISTS` statement for item `id` of the user.
pub(crate) async fn item_exists(
    conn: &mut PgConnection,
    sql: &str,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, AuthError> {
    let (exists,): (bool,) = sqlx::query_as(sql)
        .bind(id)
        .bind(user_id)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(exists)
}

/// Run a `DELETE_*` statement for item `id` of the user.
pub(crate) async fn delete_item(
    conn: &mut PgConnection,
    sql: &str,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, AuthError> {
    let result = sqlx::query(sql)
        .bind(id)
        .bind(user_id)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

// ── Phones ────────────────────────────────────────────────────────────────────

/// A user's phones, the primary first, then oldest first.
pub async fn list_phones(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<Phone>, AuthError> {
    let rows: Vec<PhoneRow> = sqlx::query_as(q::LIST_PHONES)
        .bind(user_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Insert a phone, clearing the previous primary when `phone.is_primary`.
/// Several statements: run it in a transaction.
///
/// Errors: [`AuthError::UserNotFound`], [`AuthError::PhoneAlreadyAdded`].
pub async fn add_phone(
    conn: &mut PgConnection,
    user_id: Uuid,
    phone: &NewPhone,
) -> Result<Phone, AuthError> {
    if !lock_user(conn, user_id).await? {
        return Err(AuthError::UserNotFound);
    }
    if phone.is_primary {
        clear_primary(conn, q::CLEAR_PRIMARY_PHONE, user_id, None).await?;
    }
    let row: PhoneRow = sqlx::query_as(q::INSERT_PHONE)
        .bind(user_id)
        .bind(&phone.label)
        .bind(&phone.number)
        .bind(phone.is_primary)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Replace a phone's label and number; `phone.is_primary` makes it the
/// primary one, `false` keeps its flag.  `Ok(None)`, changing nothing, if
/// the user has no such phone.  Several statements: run it in a
/// transaction.
///
/// Errors: [`AuthError::PhoneAlreadyAdded`].
pub async fn replace_phone(
    conn: &mut PgConnection,
    user_id: Uuid,
    phone_id: Uuid,
    phone: &NewPhone,
) -> Result<Option<Phone>, AuthError> {
    if !lock_user(conn, user_id).await? {
        return Ok(None);
    }
    let row: Option<PhoneRow> = sqlx::query_as(q::REPLACE_PHONE)
        .bind(phone_id)
        .bind(user_id)
        .bind(&phone.label)
        .bind(&phone.number)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    match row {
        Some(row) if phone.is_primary && !row.is_primary => {
            mark_primary_phone(conn, user_id, phone_id).await.map(Some)
        }
        row => Ok(row.map(Into::into)),
    }
}

/// Make a phone the user's primary one.  `Ok(false)`, changing nothing, if
/// the user has no such phone.  Several statements: run it in a
/// transaction.
pub async fn set_primary_phone(
    conn: &mut PgConnection,
    user_id: Uuid,
    phone_id: Uuid,
) -> Result<bool, AuthError> {
    if !lock_user(conn, user_id).await?
        || !item_exists(conn, q::PHONE_EXISTS, phone_id, user_id).await?
    {
        return Ok(false);
    }
    mark_primary_phone(conn, user_id, phone_id).await?;
    Ok(true)
}

/// Clear the previous primary, then flag the existing phone `phone_id`.
async fn mark_primary_phone(
    conn: &mut PgConnection,
    user_id: Uuid,
    phone_id: Uuid,
) -> Result<Phone, AuthError> {
    clear_primary(conn, q::CLEAR_PRIMARY_PHONE, user_id, Some(phone_id)).await?;
    let row: PhoneRow = sqlx::query_as(q::SET_PRIMARY_PHONE)
        .bind(phone_id)
        .bind(user_id)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Delete a phone.  `Ok(false)` if the user has no such phone.
pub async fn delete_phone(
    conn: &mut PgConnection,
    user_id: Uuid,
    phone_id: Uuid,
) -> Result<bool, AuthError> {
    delete_item(conn, q::DELETE_PHONE, phone_id, user_id).await
}

// ── Addresses ─────────────────────────────────────────────────────────────────

/// A user's addresses, the primary first, then oldest first.
pub async fn list_addresses(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<Address>, AuthError> {
    let rows: Vec<AddressRow> = sqlx::query_as(q::LIST_ADDRESSES)
        .bind(user_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Insert an address, clearing the previous primary when
/// `address.is_primary`.  Several statements: run it in a transaction.
///
/// Errors: [`AuthError::UserNotFound`].
pub async fn add_address(
    conn: &mut PgConnection,
    user_id: Uuid,
    address: &NewAddress,
) -> Result<Address, AuthError> {
    if !lock_user(conn, user_id).await? {
        return Err(AuthError::UserNotFound);
    }
    if address.is_primary {
        clear_primary(conn, q::CLEAR_PRIMARY_ADDRESS, user_id, None).await?;
    }
    let row: AddressRow = sqlx::query_as(q::INSERT_ADDRESS)
        .bind(user_id)
        .bind(&address.label)
        .bind(&address.line1)
        .bind(&address.line2)
        .bind(&address.locality)
        .bind(&address.region)
        .bind(&address.postal_code)
        .bind(&address.country_code)
        .bind(address.is_primary)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Replace every field of an address; `address.is_primary` makes it the
/// primary one, `false` keeps its flag.  `Ok(None)`, changing nothing, if
/// the user has no such address.  Several statements: run it in a
/// transaction.
pub async fn replace_address(
    conn: &mut PgConnection,
    user_id: Uuid,
    address_id: Uuid,
    address: &NewAddress,
) -> Result<Option<Address>, AuthError> {
    if !lock_user(conn, user_id).await? {
        return Ok(None);
    }
    let row: Option<AddressRow> = sqlx::query_as(q::REPLACE_ADDRESS)
        .bind(address_id)
        .bind(user_id)
        .bind(&address.label)
        .bind(&address.line1)
        .bind(&address.line2)
        .bind(&address.locality)
        .bind(&address.region)
        .bind(&address.postal_code)
        .bind(&address.country_code)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    match row {
        Some(row) if address.is_primary && !row.is_primary => {
            mark_primary_address(conn, user_id, address_id)
                .await
                .map(Some)
        }
        row => Ok(row.map(Into::into)),
    }
}

/// Make an address the user's primary one.  `Ok(false)`, changing nothing,
/// if the user has no such address.  Several statements: run it in a
/// transaction.
pub async fn set_primary_address(
    conn: &mut PgConnection,
    user_id: Uuid,
    address_id: Uuid,
) -> Result<bool, AuthError> {
    if !lock_user(conn, user_id).await?
        || !item_exists(conn, q::ADDRESS_EXISTS, address_id, user_id).await?
    {
        return Ok(false);
    }
    mark_primary_address(conn, user_id, address_id).await?;
    Ok(true)
}

/// Clear the previous primary, then flag the existing address `address_id`.
async fn mark_primary_address(
    conn: &mut PgConnection,
    user_id: Uuid,
    address_id: Uuid,
) -> Result<Address, AuthError> {
    clear_primary(conn, q::CLEAR_PRIMARY_ADDRESS, user_id, Some(address_id)).await?;
    let row: AddressRow = sqlx::query_as(q::SET_PRIMARY_ADDRESS)
        .bind(address_id)
        .bind(user_id)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Delete an address.  `Ok(false)` if the user has no such address.
pub async fn delete_address(
    conn: &mut PgConnection,
    user_id: Uuid,
    address_id: Uuid,
) -> Result<bool, AuthError> {
    delete_item(conn, q::DELETE_ADDRESS, address_id, user_id).await
}
