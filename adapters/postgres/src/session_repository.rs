//! PostgreSQL implementation of [`SessionRepository`].
//!
//! The statements live in [`crate::tx::sessions`].  Every timestamp is
//! assigned by the database (`now()`); only durations are bound.  `create`
//! and `rotate` run in transactions; `rotate` is a compare-and-swap on
//! `current_generation`, so concurrent refreshes of one session are settled
//! by the row lock (the loser gets [`RotateOutcome::Conflict`]).

use std::time::Duration;

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::security::session::RevocationReason;
use z_auth::security::session::{
    IssueContext, NewSession, RefreshSnapshot, RotateOutcome, Session, SessionGeneration,
    SessionLifetimes, SessionRepository, SessionStatus,
};

use crate::errors::map_sqlx_error;
use crate::tx::sessions;

/// PostgreSQL-backed session repository.
pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    async fn create(
        &self,
        session: NewSession,
        lifetimes: &SessionLifetimes,
    ) -> Result<(Session, SessionGeneration), AuthError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let created = sessions::create(&mut tx, session, lifetimes).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(created)
    }

    async fn find(&self, session_id: Uuid) -> Result<Option<Session>, AuthError> {
        sessions::find(&mut *self.conn().await?, session_id).await
    }

    async fn find_generation(
        &self,
        session_id: Uuid,
        generation: u32,
    ) -> Result<Option<SessionGeneration>, AuthError> {
        sessions::find_generation(&mut *self.conn().await?, session_id, generation).await
    }

    async fn load_for_refresh(
        &self,
        session_id: Uuid,
        generation: u32,
    ) -> Result<Option<RefreshSnapshot>, AuthError> {
        sessions::load_for_refresh(&mut *self.conn().await?, session_id, generation).await
    }

    async fn rotate(
        &self,
        session_id: Uuid,
        expected_generation: u32,
        issued: &IssueContext,
        lifetimes: &SessionLifetimes,
    ) -> Result<RotateOutcome, AuthError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let outcome =
            sessions::rotate(&mut tx, session_id, expected_generation, issued, lifetimes).await?;
        match outcome {
            RotateOutcome::Conflict => tx.rollback().await.map_err(map_sqlx_error)?,
            RotateOutcome::Rotated { .. } => tx.commit().await.map_err(map_sqlx_error)?,
        }
        Ok(outcome)
    }

    async fn end(
        &self,
        session_id: Uuid,
        status: SessionStatus,
        reason: RevocationReason,
    ) -> Result<bool, AuthError> {
        sessions::end(&mut *self.conn().await?, session_id, status, reason).await
    }

    async fn mark_compromised(
        &self,
        session_id: Uuid,
        reason: RevocationReason,
    ) -> Result<bool, AuthError> {
        sessions::mark_compromised(&mut *self.conn().await?, session_id, reason).await
    }

    async fn end_all_for_user(
        &self,
        user_id: Uuid,
        status: SessionStatus,
        reason: RevocationReason,
    ) -> Result<Vec<Uuid>, AuthError> {
        sessions::end_all_for_user(&mut *self.conn().await?, user_id, status, reason).await
    }

    async fn list_active_for_user(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError> {
        sessions::list_active_for_user(&mut *self.conn().await?, user_id).await
    }

    async fn purge(&self, retention: Duration) -> Result<u64, AuthError> {
        sessions::purge(&mut *self.conn().await?, retention).await
    }
}
