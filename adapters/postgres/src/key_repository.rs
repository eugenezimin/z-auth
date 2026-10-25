//! PostgreSQL implementation of [`KeyRepository`]: published verifying
//! keys.  The statements live in [`crate::tx::keys`].

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use z_auth::AuthError;
use z_auth::security::token::{KeyRepository, NewVerifyingKey, VerifyingKeyRecord};

use crate::errors::map_sqlx_error;
use crate::tx::keys;

/// PostgreSQL-backed store of published verifying keys.
pub struct PgKeyRepository {
    pool: PgPool,
}

impl PgKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl KeyRepository for PgKeyRepository {
    async fn publish(&self, key: &NewVerifyingKey) -> Result<VerifyingKeyRecord, AuthError> {
        keys::publish(&mut *self.conn().await?, key).await
    }

    async fn revoke(&self, kid: &str) -> Result<Option<VerifyingKeyRecord>, AuthError> {
        keys::revoke(&mut *self.conn().await?, kid).await
    }

    async fn list(&self) -> Result<Vec<VerifyingKeyRecord>, AuthError> {
        keys::list(&mut *self.conn().await?).await
    }
}
