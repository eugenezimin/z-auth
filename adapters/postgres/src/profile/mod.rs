//! User profiles and organizations (feature `profile`):
//! [`PgProfileRepository`] and [`PgOrganizationRepository`].  Their tables
//! are created by [`PROFILE_MIGRATOR`](crate::PROFILE_MIGRATOR).

mod organization_queries;
mod organization_repository;
mod profile_queries;
mod profile_repository;

pub use organization_repository::PgOrganizationRepository;
pub use profile_repository::PgProfileRepository;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use z_auth::AuthError;

use crate::errors::map_sqlx_error;

/// Open a transaction that writes one user's items: lock the user's row
/// (so primary changes for that user serialize), then run `clear_primary`
/// (`$1` = user id) when given.  `Ok(None)` if the user does not exist.
pub(crate) async fn begin_for_user(
    pool: &PgPool,
    user_id: Uuid,
    clear_primary: Option<&str>,
) -> Result<Option<Transaction<'static, Postgres>>, AuthError> {
    let mut tx = pool.begin().await.map_err(map_sqlx_error)?;
    let locked: Option<(Uuid,)> = sqlx::query_as(profile_queries::LOCK_USER)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
    if locked.is_none() {
        return Ok(None);
    }
    if let Some(clear) = clear_primary {
        sqlx::query(clear)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
    }
    Ok(Some(tx))
}

/// Make item `id` the user's primary one: clear the previous primary with
/// `clear`, then flag the item with `set` (`$1` = item id, `$2` = user id).
/// Returns `false`, changing nothing, if the user has no such item.
pub(crate) async fn set_primary(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    clear: &str,
    set: &str,
) -> Result<bool, AuthError> {
    let Some(mut tx) = begin_for_user(pool, user_id, Some(clear)).await? else {
        return Ok(false);
    };
    let flagged = sqlx::query(set)
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
    if flagged.rows_affected() == 0 {
        return Ok(false); // dropping `tx` rolls the clearing back
    }
    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(true)
}

/// Delete item `id` of the user with `delete` (`$1` = item id, `$2` = user
/// id).  Returns `false` if the user has no such item.
pub(crate) async fn delete_item(
    pool: &PgPool,
    user_id: Uuid,
    id: Uuid,
    delete: &str,
) -> Result<bool, AuthError> {
    let result = sqlx::query(delete)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(map_sqlx_error)?;
    Ok(result.rows_affected() > 0)
}
