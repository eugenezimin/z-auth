//! Published verifying keys: the SQL behind
//! [`PgKeyRepository`](crate::PgKeyRepository), callable on any connection.

use sqlx::PgConnection;
use z_auth::AuthError;
use z_auth::security::token::{NewVerifyingKey, VerifyingKeyRecord};

use crate::errors::map_sqlx_error;
use crate::queries::key_queries as q;
use crate::rows::KeyRow;

/// Publish a verifying key.
pub async fn publish(
    conn: &mut PgConnection,
    key: &NewVerifyingKey,
) -> Result<VerifyingKeyRecord, AuthError> {
    let row: KeyRow = sqlx::query_as(q::PUBLISH_KEY)
        .bind(&key.kid)
        .bind(key.public_key.as_slice())
        .bind(key.published_by)
        .fetch_one(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.try_into()
}

/// Revoke a key.  `None` when there is no such key.
pub async fn revoke(
    conn: &mut PgConnection,
    kid: &str,
) -> Result<Option<VerifyingKeyRecord>, AuthError> {
    let row: Option<KeyRow> = sqlx::query_as(q::REVOKE_KEY)
        .bind(kid)
        .fetch_optional(conn)
        .await
        .map_err(map_sqlx_error)?;
    row.map(TryInto::try_into).transpose()
}

/// Every published key, revoked ones included.
pub async fn list(conn: &mut PgConnection) -> Result<Vec<VerifyingKeyRecord>, AuthError> {
    let rows: Vec<KeyRow> = sqlx::query_as(q::LIST_KEYS)
        .fetch_all(conn)
        .await
        .map_err(map_sqlx_error)?;
    rows.into_iter().map(TryInto::try_into).collect()
}
