//! PostgreSQL implementation of [`UserRepository`] using `sqlx`.
//!
//! The statements live in [`crate::tx::users`]; this type runs them on a
//! pooled connection.  Uses the non-macro `sqlx::query_as` API so no
//! `DATABASE_URL` is required at compile time.

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::user::{
    NewUser, User, UserPage, UserQuery, UserRepository, UserUpdate, UserWithRoles,
};

use crate::errors::map_sqlx_error;
use crate::tx::users;

/// PostgreSQL-backed user repository.
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, AuthError> {
        users::find_by_id(&mut *self.conn().await?, user_id).await
    }
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        users::find_by_email(&mut *self.conn().await?, email).await
    }
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AuthError> {
        users::find_by_username(&mut *self.conn().await?, username).await
    }

    async fn find_with_roles_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserWithRoles>, AuthError> {
        users::find_with_roles_by_id(&mut *self.conn().await?, user_id).await
    }
    async fn find_with_roles_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserWithRoles>, AuthError> {
        users::find_with_roles_by_email(&mut *self.conn().await?, email).await
    }
    async fn find_with_roles_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserWithRoles>, AuthError> {
        users::find_with_roles_by_username(&mut *self.conn().await?, username).await
    }

    async fn list(&self, query: &UserQuery) -> Result<UserPage, AuthError> {
        users::list(&mut *self.conn().await?, query).await
    }

    async fn exists_by_email(&self, email: &str) -> Result<bool, AuthError> {
        users::exists_by_email(&mut *self.conn().await?, email).await
    }
    async fn exists_by_username(&self, username: &str) -> Result<bool, AuthError> {
        users::exists_by_username(&mut *self.conn().await?, username).await
    }

    async fn create(&self, new_user: NewUser) -> Result<User, AuthError> {
        users::create(&mut *self.conn().await?, &new_user).await
    }

    async fn update(&self, user_id: Uuid, update: UserUpdate) -> Result<Option<User>, AuthError> {
        users::update(&mut *self.conn().await?, user_id, &update).await
    }

    async fn delete(&self, user_id: Uuid) -> Result<Option<Uuid>, AuthError> {
        users::delete(&mut *self.conn().await?, user_id).await
    }

    async fn activate(&self, user_id: Uuid) -> Result<bool, AuthError> {
        users::activate(&mut *self.conn().await?, user_id).await
    }
    async fn deactivate(&self, user_id: Uuid) -> Result<bool, AuthError> {
        users::deactivate(&mut *self.conn().await?, user_id).await
    }
    async fn is_active(&self, user_id: Uuid) -> Result<Option<bool>, AuthError> {
        users::is_active(&mut *self.conn().await?, user_id).await
    }
    async fn is_verified(&self, user_id: Uuid) -> Result<Option<bool>, AuthError> {
        users::is_verified(&mut *self.conn().await?, user_id).await
    }
}
