//! The adapter's SQL as free functions over a borrowed connection, for hosts
//! that write z-auth state and their own state in **one transaction** (for
//! example, a user change together with a transactional-outbox row).
//!
//! Every repository in this crate is a thin wrapper around these functions,
//! so both paths run the same statements and map errors the same way.
//!
//! ```rust,ignore
//! let mut tx = pool.begin().await?;
//! let user = z_auth_postgres::tx::users::create(&mut tx, &new_user).await?;
//! z_auth_postgres::tx::user_roles::assign(&mut tx, user.id, role.id).await?;
//! sqlx::query("INSERT INTO outbox …").execute(&mut *tx).await?;
//! tx.commit().await?;
//! ```
//!
//! Functions documented as running several statements must be called inside
//! a transaction; the rest are single statements.  Queries are unqualified:
//! the connection's `search_path` selects the schema.

pub mod keys;
#[cfg(feature = "profile")]
pub mod organizations;
pub mod permissions;
#[cfg(feature = "profile")]
pub mod profiles;
pub mod revocations;
pub mod roles;
pub mod sessions;
pub mod user_roles;
pub mod users;
