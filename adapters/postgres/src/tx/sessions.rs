//! Sessions: the SQL behind [`PgSessionRepository`](crate::PgSessionRepository),
//! callable on any connection.  Every timestamp is assigned by the database
//! (`now()`); only durations are bound.
//!
//! [`create`] and [`rotate`] run several statements: call them inside a
//! transaction (the repository does).  `rotate` is a compare-and-swap on
//! `current_generation`, so concurrent refreshes of one session are settled
//! by the row lock (the loser gets [`RotateOutcome::Conflict`]).

use std::time::Duration;

use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::security::session::{
    IssueContext, NewSession, RefreshSnapshot, RevocationReason, RotateOutcome, Session,
    SessionGeneration, SessionLifetimes, SessionStatus,
};

use crate::codes::to_db_generation;
use crate::enums::{PgRevocationReason, PgSessionStatus};
use crate::errors::map_sqlx_error;
use crate::queries::session_queries as q;
use crate::rows::{GenerationRow, RefreshRow, SessionRow};

async fn insert_generation(
    conn: &mut PgConnection,
    session_id: Uuid,
    generation: i32,
    issued: &IssueContext,
    access_ttl: Duration,
) -> Result<SessionGeneration, AuthError> {
    let row: GenerationRow = sqlx::query_as(q::INSERT_GENERATION)
        .bind(session_id)
        .bind(generation)
        .bind(issued.ip)
        .bind(access_ttl)
        .bind(issued.parent_jti)
        .bind(&issued.user_agent)
        .bind(issued.fingerprint_hash.map(|h| h.to_vec()))
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.try_into()
}

/// Open a session with its first generation.  Several statements: run it
/// in a transaction.
pub async fn create(
    conn: &mut PgConnection,
    session: NewSession,
    lifetimes: &SessionLifetimes,
) -> Result<(Session, SessionGeneration), AuthError> {
    let new_fingerprint = session.fingerprint_hash;
    let row: SessionRow = sqlx::query_as(q::INSERT_SESSION)
        .bind(session.user_id)
        .bind(&session.secret)
        .bind(session.created_ip)
        .bind(&session.user_agent)
        .bind(lifetimes.idle_timeout)
        .bind(lifetimes.absolute_timeout)
        .bind(&session.amr)
        .fetch_one(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let session: Session = row.try_into()?;
    let first = insert_generation(
        conn,
        session.id,
        to_db_generation(session.current_generation)?,
        &IssueContext {
            ip: session.created_ip,
            user_agent: session.user_agent.clone(),
            fingerprint_hash: new_fingerprint,
            parent_jti: None,
        },
        lifetimes.access_ttl,
    )
    .await?;
    Ok((session, first))
}

/// The session with this id.
pub async fn find(conn: &mut PgConnection, session_id: Uuid) -> Result<Option<Session>, AuthError> {
    let row: Option<SessionRow> = sqlx::query_as(q::FIND_SESSION)
        .bind(session_id)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.map(TryInto::try_into).transpose()
}

/// One generation of a session.
pub async fn find_generation(
    conn: &mut PgConnection,
    session_id: Uuid,
    generation: u32,
) -> Result<Option<SessionGeneration>, AuthError> {
    let row: Option<GenerationRow> = sqlx::query_as(q::FIND_GENERATION)
        .bind(session_id)
        .bind(to_db_generation(generation)?)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.map(TryInto::try_into).transpose()
}

/// Everything a refresh decision needs, in one read.
pub async fn load_for_refresh(
    conn: &mut PgConnection,
    session_id: Uuid,
    generation: u32,
) -> Result<Option<RefreshSnapshot>, AuthError> {
    let row: Option<RefreshRow> = sqlx::query_as(q::LOAD_FOR_REFRESH)
        .bind(session_id)
        .bind(to_db_generation(generation)?)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.map(TryInto::try_into).transpose()
}

/// Advance a session to its next generation if it is still at
/// `expected_generation`.  Several statements: run it in a transaction, and
/// roll back on [`RotateOutcome::Conflict`].
pub async fn rotate(
    conn: &mut PgConnection,
    session_id: Uuid,
    expected_generation: u32,
    issued: &IssueContext,
    lifetimes: &SessionLifetimes,
) -> Result<RotateOutcome, AuthError> {
    let expected = to_db_generation(expected_generation)?;
    let newest = to_db_generation(expected_generation.saturating_add(1))?;

    let advanced: Option<SessionRow> = sqlx::query_as(q::ADVANCE_SESSION)
        .bind(session_id)
        .bind(expected)
        .bind(lifetimes.idle_timeout)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let Some(session) = advanced else {
        return Ok(RotateOutcome::Conflict);
    };

    sqlx::query(q::SUPERSEDE_GENERATION)
        .bind(session_id)
        .bind(expected)
        .execute(&mut *conn)
        .await
        .map_err(map_sqlx_error)?;
    let generation =
        insert_generation(conn, session_id, newest, issued, lifetimes.access_ttl).await?;
    let history = i32::try_from(lifetimes.history_size).unwrap_or(i32::MAX);
    let oldest_dropped = newest.saturating_sub(history);
    for trim in [q::TRIM_GENERATIONS, q::TRIM_ALIASES] {
        sqlx::query(trim)
            .bind(session_id)
            .bind(oldest_dropped)
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx_error)?;
    }
    Ok(RotateOutcome::Rotated {
        session: session.try_into()?,
        generation,
    })
}

/// End one session.  `false` when it was not active.
pub async fn end(
    conn: &mut PgConnection,
    session_id: Uuid,
    status: SessionStatus,
    reason: RevocationReason,
) -> Result<bool, AuthError> {
    let result = sqlx::query(q::END_SESSION)
        .bind(session_id)
        .bind(PgSessionStatus::from(status))
        .bind(PgRevocationReason::from(reason))
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

/// Mark a session compromised (refresh-token reuse).
pub async fn mark_compromised(
    conn: &mut PgConnection,
    session_id: Uuid,
    reason: RevocationReason,
) -> Result<bool, AuthError> {
    let result = sqlx::query(q::MARK_COMPROMISED)
        .bind(session_id)
        .bind(PgRevocationReason::from(reason))
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}

/// End every active session of the user; returns their ids.
pub async fn end_all_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
    status: SessionStatus,
    reason: RevocationReason,
) -> Result<Vec<Uuid>, AuthError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(q::END_ALL_FOR_USER)
        .bind(user_id)
        .bind(PgSessionStatus::from(status))
        .bind(PgRevocationReason::from(reason))
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// The user's active sessions.
pub async fn list_active_for_user(
    conn: &mut PgConnection,
    user_id: Uuid,
) -> Result<Vec<Session>, AuthError> {
    let rows: Vec<SessionRow> = sqlx::query_as(q::LIST_ACTIVE_FOR_USER)
        .bind(user_id)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    rows.into_iter().map(TryInto::try_into).collect()
}

/// Delete sessions that ended more than `retention` ago; returns how many.
pub async fn purge(conn: &mut PgConnection, retention: Duration) -> Result<u64, AuthError> {
    let result = sqlx::query(q::PURGE_SESSIONS)
        .bind(retention)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected())
}
