//! PostgreSQL implementation of z-auth's [`ProfileRepository`].
//!
//! The statements live in [`crate::tx::profiles`].

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::profile::{Address, NewAddress, NewPhone, Phone, ProfileRepository};

use crate::errors::map_sqlx_error;
use crate::tx::profiles;

/// PostgreSQL-backed phones and addresses (see
/// `migrations_profile/4001_profile_schema.sql`).
pub struct PgProfileRepository {
    pool: PgPool,
}

impl PgProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

/// Run a several-statement function of [`profiles`] in its own transaction.
macro_rules! in_tx {
    ($pool:expr, |$tx:ident| $call:expr) => {{
        let mut $tx = $pool.begin().await.map_err(map_sqlx_error)?;
        let out = $call.await?;
        $tx.commit().await.map_err(map_sqlx_error)?;
        Ok(out)
    }};
}
pub(crate) use in_tx;

#[async_trait]
impl ProfileRepository for PgProfileRepository {
    async fn list_phones(&self, user_id: Uuid) -> Result<Vec<Phone>, AuthError> {
        profiles::list_phones(&mut *self.conn().await?, user_id).await
    }

    async fn add_phone(&self, user_id: Uuid, phone: NewPhone) -> Result<Phone, AuthError> {
        in_tx!(self.pool, |tx| profiles::add_phone(
            &mut tx, user_id, &phone
        ))
    }

    async fn replace_phone(
        &self,
        user_id: Uuid,
        phone_id: Uuid,
        phone: NewPhone,
    ) -> Result<Option<Phone>, AuthError> {
        in_tx!(self.pool, |tx| profiles::replace_phone(
            &mut tx, user_id, phone_id, &phone
        ))
    }

    async fn set_primary_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<bool, AuthError> {
        in_tx!(self.pool, |tx| profiles::set_primary_phone(
            &mut tx, user_id, phone_id
        ))
    }

    async fn delete_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<bool, AuthError> {
        profiles::delete_phone(&mut *self.conn().await?, user_id, phone_id).await
    }

    async fn list_addresses(&self, user_id: Uuid) -> Result<Vec<Address>, AuthError> {
        profiles::list_addresses(&mut *self.conn().await?, user_id).await
    }

    async fn add_address(&self, user_id: Uuid, address: NewAddress) -> Result<Address, AuthError> {
        in_tx!(self.pool, |tx| profiles::add_address(
            &mut tx, user_id, &address
        ))
    }

    async fn replace_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        address: NewAddress,
    ) -> Result<Option<Address>, AuthError> {
        in_tx!(self.pool, |tx| profiles::replace_address(
            &mut tx, user_id, address_id, &address
        ))
    }

    async fn set_primary_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
    ) -> Result<bool, AuthError> {
        in_tx!(self.pool, |tx| profiles::set_primary_address(
            &mut tx, user_id, address_id
        ))
    }

    async fn delete_address(&self, user_id: Uuid, address_id: Uuid) -> Result<bool, AuthError> {
        profiles::delete_address(&mut *self.conn().await?, user_id, address_id).await
    }
}
