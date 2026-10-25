//! PostgreSQL implementation of [`RevocationRepository`] — the persisted
//! denylist.  The statements live in [`crate::tx::revocations`];
//! `revoked_at` / `expires_at` are stamped by the database.

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::security::token::{NewRevocation, Revocation, RevocationRepository};

use crate::errors::map_sqlx_error;
use crate::tx::revocations;

/// PostgreSQL-backed revocation repository.
pub struct PgRevocationRepository {
    pool: PgPool,
}

impl PgRevocationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl RevocationRepository for PgRevocationRepository {
    async fn insert(&self, revocation: &NewRevocation) -> Result<Revocation, AuthError> {
        revocations::insert(&mut *self.conn().await?, revocation).await
    }

    async fn mark_enforced(&self, id: Uuid, by: Uuid) -> Result<Option<Revocation>, AuthError> {
        revocations::mark_enforced(&mut *self.conn().await?, id, by).await
    }

    async fn list_active(&self) -> Result<Vec<Revocation>, AuthError> {
        revocations::list_active(&mut *self.conn().await?).await
    }

    async fn purge_expired(&self) -> Result<u64, AuthError> {
        revocations::purge_expired(&mut *self.conn().await?).await
    }
}
