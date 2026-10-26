//! PostgreSQL implementation of [`UserRoleRepository`] using `sqlx`.
//!
//! The statements live in [`crate::tx::user_roles`].

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::access::{UserRole, UserRoleRepository};

use crate::errors::map_sqlx_error;
use crate::tx::user_roles;

/// PostgreSQL-backed user → role assignment repository.
pub struct PgUserRoleRepository {
    pool: PgPool,
}

impl PgUserRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl UserRoleRepository for PgUserRoleRepository {
    async fn assign(&self, user_id: Uuid, role_id: Uuid) -> Result<bool, AuthError> {
        user_roles::assign(&mut *self.conn().await?, user_id, role_id).await
    }

    async fn revoke(&self, user_id: Uuid, role_id: Uuid) -> Result<bool, AuthError> {
        user_roles::revoke(&mut *self.conn().await?, user_id, role_id).await
    }

    async fn is_role_active(&self, user_id: Uuid, role_id: Uuid) -> Result<bool, AuthError> {
        user_roles::is_role_active(&mut *self.conn().await?, user_id, role_id).await
    }

    async fn list_active_for_user(&self, user_id: Uuid) -> Result<Vec<UserRole>, AuthError> {
        user_roles::list_active_for_user(&mut *self.conn().await?, user_id).await
    }

    async fn list_all_for_user(&self, user_id: Uuid) -> Result<Vec<UserRole>, AuthError> {
        user_roles::list_all_for_user(&mut *self.conn().await?, user_id).await
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64, AuthError> {
        user_roles::revoke_all_for_user(&mut *self.conn().await?, user_id).await
    }

    async fn list_active_holders(&self, role_id: Uuid) -> Result<Vec<Uuid>, AuthError> {
        user_roles::list_active_holders(&mut *self.conn().await?, role_id).await
    }
}
