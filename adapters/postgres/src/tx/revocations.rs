//! Revocations (the persisted denylist): the SQL behind
//! [`PgRevocationRepository`](crate::PgRevocationRepository), callable on any
//! connection.  `revoked_at` / `expires_at` are stamped by the database.

use sqlx::PgConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::security::token::{NewRevocation, Revocation, RevocationScope};

use crate::enums::{PgRevocationReason, PgRevocationScope};
use crate::errors::map_sqlx_error;
use crate::queries::revocation_queries as q;
use crate::rows::RevocationRow;

/// Record a revocation.
pub async fn insert(
    conn: &mut PgConnection,
    revocation: &NewRevocation,
) -> Result<Revocation, AuthError> {
    let (scope, subject) = match revocation.scope {
        RevocationScope::Session(id) => (PgRevocationScope::Session, id),
        RevocationScope::User(id) => (PgRevocationScope::User, id),
        RevocationScope::Token(jti) => (PgRevocationScope::Token, jti),
    };
    let row: RevocationRow = sqlx::query_as(q::INSERT_REVOCATION)
        .bind(scope)
        .bind(subject)
        .bind(PgRevocationReason::from(revocation.reason))
        .bind(revocation.origin_node)
        .bind(revocation.ttl)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.into())
}

/// Compare-and-swap a pending revocation to enforced.  `None` when it was
/// not pending (already enforced, or unknown).
pub async fn mark_enforced(
    conn: &mut PgConnection,
    id: Uuid,
    by: Uuid,
) -> Result<Option<Revocation>, AuthError> {
    let row: Option<RevocationRow> = sqlx::query_as(q::MARK_ENFORCED)
        .bind(id)
        .bind(by)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(row.map(Into::into))
}

/// Every unexpired revocation.
pub async fn list_active(conn: &mut PgConnection) -> Result<Vec<Revocation>, AuthError> {
    let rows: Vec<RevocationRow> = sqlx::query_as(q::LIST_ACTIVE)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Delete expired revocations; returns how many.
pub async fn purge_expired(conn: &mut PgConnection) -> Result<u64, AuthError> {
    let result = sqlx::query(q::PURGE_EXPIRED)
        .execute(conn)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected())
}
