//! PostgreSQL implementation of z-auth's [`ProfileRepository`].  All SQL
//! lives in [`profile_queries`](super::profile_queries).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::profile::{Address, NewAddress, NewPhone, Phone, ProfileRepository};

use super::profile_queries as q;
use super::{begin_for_user, delete_item, set_primary};
use crate::errors::map_sqlx_error;

/// PostgreSQL-backed phones and addresses (see
/// `migrations_profile/4001_profile_schema.sql`).
pub struct PgProfileRepository {
    pool: PgPool,
}

impl PgProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

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

/// The primary-clearing query to run first, if `is_primary` is requested.
fn clear_if(is_primary: bool, clear: &'static str) -> Option<&'static str> {
    is_primary.then_some(clear)
}

#[async_trait]
impl ProfileRepository for PgProfileRepository {
    async fn list_phones(&self, user_id: Uuid) -> Result<Vec<Phone>, AuthError> {
        let rows: Vec<PhoneRow> = sqlx::query_as(q::LIST_PHONES)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn add_phone(&self, user_id: Uuid, phone: NewPhone) -> Result<Phone, AuthError> {
        let clear = clear_if(phone.is_primary, q::CLEAR_PRIMARY_PHONE);
        let mut tx = begin_for_user(&self.pool, user_id, clear)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        let row: PhoneRow = sqlx::query_as(q::INSERT_PHONE)
            .bind(user_id)
            .bind(&phone.label)
            .bind(&phone.number)
            .bind(phone.is_primary)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(row.into())
    }

    async fn replace_phone(
        &self,
        user_id: Uuid,
        phone_id: Uuid,
        phone: NewPhone,
    ) -> Result<Option<Phone>, AuthError> {
        let clear = clear_if(phone.is_primary, q::CLEAR_PRIMARY_PHONE);
        let Some(mut tx) = begin_for_user(&self.pool, user_id, clear).await? else {
            return Ok(None);
        };
        let row: Option<PhoneRow> = sqlx::query_as(q::REPLACE_PHONE)
            .bind(phone_id)
            .bind(user_id)
            .bind(&phone.label)
            .bind(&phone.number)
            .bind(phone.is_primary)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        // No such phone: dropping `tx` rolls the clearing back.
        if row.is_some() {
            tx.commit().await.map_err(map_sqlx_error)?;
        }
        Ok(row.map(Into::into))
    }

    async fn set_primary_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<bool, AuthError> {
        set_primary(
            &self.pool,
            user_id,
            phone_id,
            q::CLEAR_PRIMARY_PHONE,
            q::SET_PRIMARY_PHONE,
        )
        .await
    }

    async fn delete_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<bool, AuthError> {
        delete_item(&self.pool, user_id, phone_id, q::DELETE_PHONE).await
    }

    async fn list_addresses(&self, user_id: Uuid) -> Result<Vec<Address>, AuthError> {
        let rows: Vec<AddressRow> = sqlx::query_as(q::LIST_ADDRESSES)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn add_address(&self, user_id: Uuid, address: NewAddress) -> Result<Address, AuthError> {
        let clear = clear_if(address.is_primary, q::CLEAR_PRIMARY_ADDRESS);
        let mut tx = begin_for_user(&self.pool, user_id, clear)
            .await?
            .ok_or(AuthError::UserNotFound)?;
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
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(row.into())
    }

    async fn replace_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        address: NewAddress,
    ) -> Result<Option<Address>, AuthError> {
        let clear = clear_if(address.is_primary, q::CLEAR_PRIMARY_ADDRESS);
        let Some(mut tx) = begin_for_user(&self.pool, user_id, clear).await? else {
            return Ok(None);
        };
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
            .bind(address.is_primary)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        if row.is_some() {
            tx.commit().await.map_err(map_sqlx_error)?;
        }
        Ok(row.map(Into::into))
    }

    async fn set_primary_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
    ) -> Result<bool, AuthError> {
        set_primary(
            &self.pool,
            user_id,
            address_id,
            q::CLEAR_PRIMARY_ADDRESS,
            q::SET_PRIMARY_ADDRESS,
        )
        .await
    }

    async fn delete_address(&self, user_id: Uuid, address_id: Uuid) -> Result<bool, AuthError> {
        delete_item(&self.pool, user_id, address_id, q::DELETE_ADDRESS).await
    }
}
