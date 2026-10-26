//! PostgreSQL implementation of [`RoleRepository`] using `sqlx`.
//!
//! The statements live in [`crate::tx::roles`].

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::access::{NewRole, Role, RoleRepository, RoleUpdate};

use crate::errors::map_sqlx_error;
use crate::tx::roles;

/// PostgreSQL-backed role repository.
pub struct PgRoleRepository {
    pool: PgPool,
}

impl PgRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl RoleRepository for PgRoleRepository {
    async fn create(&self, new_role: &NewRole) -> Result<Role, AuthError> {
        roles::create(&mut *self.conn().await?, new_role).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Role>, AuthError> {
        roles::find_by_id(&mut *self.conn().await?, id).await
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, AuthError> {
        roles::find_by_name(&mut *self.conn().await?, name).await
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Role>, AuthError> {
        roles::find_by_code(&mut *self.conn().await?, code).await
    }

    async fn list_all(&self) -> Result<Vec<Role>, AuthError> {
        roles::list_all(&mut *self.conn().await?).await
    }

    async fn update(&self, id: Uuid, update: &RoleUpdate) -> Result<Option<Role>, AuthError> {
        roles::update(&mut *self.conn().await?, id, update).await
    }

    async fn delete(&self, id: Uuid) -> Result<Option<Uuid>, AuthError> {
        roles::delete(&mut *self.conn().await?, id).await
    }

    async fn exists_by_name(&self, name: &str) -> Result<bool, AuthError> {
        roles::exists_by_name(&mut *self.conn().await?, name).await
    }
}
