//! PostgreSQL implementation of [`PermissionRepository`] — the permission
//! catalog and user / role grants.  The statements live in
//! [`crate::tx::permissions`].
//!
//! Called by z-auth only for admin operations, at startup (catalog load)
//! and at login / refresh (`effective_grants`) — never on token verification.

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::security::authorization::{
    EffectiveGrants, NewPermission, Permission, PermissionAssignment, PermissionCatalog,
    PermissionRepository,
};

use crate::errors::map_sqlx_error;
use crate::tx::permissions;

/// PostgreSQL-backed permission repository.
pub struct PgPermissionRepository {
    pool: PgPool,
}

impl PgPermissionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl PermissionRepository for PgPermissionRepository {
    async fn create(&self, permission: &NewPermission) -> Result<Permission, AuthError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let created = permissions::create(&mut tx, permission).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(created)
    }

    async fn add_option(
        &self,
        permission_id: Uuid,
        option_code: &str,
    ) -> Result<Permission, AuthError> {
        permissions::add_option(&mut *self.conn().await?, permission_id, option_code).await
    }

    async fn delete(&self, permission_id: Uuid) -> Result<bool, AuthError> {
        permissions::delete(&mut *self.conn().await?, permission_id).await
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Permission>, AuthError> {
        permissions::find_by_code(&mut *self.conn().await?, code).await
    }

    async fn load_catalog(&self) -> Result<PermissionCatalog, AuthError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let catalog = permissions::load_catalog(&mut tx).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(catalog)
    }

    async fn replace_user_grant(
        &self,
        user_id: Uuid,
        assignment: &PermissionAssignment,
    ) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        permissions::replace_user_grant(&mut tx, user_id, assignment).await?;
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn clear_user_grant(
        &self,
        user_id: Uuid,
        permission_id: Uuid,
    ) -> Result<bool, AuthError> {
        permissions::clear_user_grant(&mut *self.conn().await?, user_id, permission_id).await
    }

    async fn replace_role_grant(
        &self,
        role_id: Uuid,
        assignment: &PermissionAssignment,
    ) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        permissions::replace_role_grant(&mut tx, role_id, assignment).await?;
        tx.commit().await.map_err(map_sqlx_error)
    }

    async fn list_user_grants(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PermissionAssignment>, AuthError> {
        permissions::list_user_grants(&mut *self.conn().await?, user_id).await
    }

    async fn list_role_grants(
        &self,
        role_id: Uuid,
    ) -> Result<Vec<PermissionAssignment>, AuthError> {
        permissions::list_role_grants(&mut *self.conn().await?, role_id).await
    }

    async fn clear_role_grant(
        &self,
        role_id: Uuid,
        permission_id: Uuid,
    ) -> Result<bool, AuthError> {
        permissions::clear_role_grant(&mut *self.conn().await?, role_id, permission_id).await
    }

    async fn effective_grants(
        &self,
        user_id: Uuid,
        include_role_grants: bool,
    ) -> Result<EffectiveGrants, AuthError> {
        permissions::effective_grants(&mut *self.conn().await?, user_id, include_role_grants).await
    }
}
