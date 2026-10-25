//! Integration tests — the `tx` module: z-auth writes that commit or roll
//! back together with the host's own statements.
//!
//! Needs a PostgreSQL database (see `helpers.rs` for the `DB_*` env vars).

mod helpers;

use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::access::NewRole;
use z_auth::identity::user::NewUser;
use z_auth::security::session::RevocationReason;
use z_auth::security::token::{NewRevocation, RevocationScope};
use z_auth_postgres::tx;

use crate::helpers::{code_of, pool, unique_email, unique_name};

fn new_user(email: &str) -> NewUser {
    NewUser {
        email: email.to_string(),
        password_hash: None,
        username: None,
        first_name: Some("Ada".into()),
        last_name: Some("Lovelace".into()),
        is_verified: false,
    }
}

fn new_role(name: &str) -> NewRole {
    NewRole {
        code: code_of(name),
        name: name.to_string(),
        description: None,
    }
}

#[tokio::test]
async fn rolled_back_transaction_leaves_no_user_role_or_assignment() {
    let pool = pool().await;
    let email = unique_email("tx-rollback");
    let role_name = unique_name("tx-role");

    let mut t = pool.begin().await.unwrap();
    let user = tx::users::create(&mut t, &new_user(&email)).await.unwrap();
    let role = tx::roles::create(&mut t, &new_role(&role_name))
        .await
        .unwrap();
    assert!(
        tx::user_roles::assign(&mut t, user.id, role.id)
            .await
            .unwrap()
    );
    // Visible inside the transaction.
    let seen = tx::users::find_with_roles_by_id(&mut t, user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(seen.roles.len(), 1);
    t.rollback().await.unwrap();

    let mut c = pool.acquire().await.unwrap();
    assert!(
        tx::users::find_by_email(&mut c, &email)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        tx::roles::find_by_name(&mut c, &role_name)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn committed_transaction_includes_host_statements() {
    let pool = pool().await;
    let email = unique_email("tx-commit");
    let id = Uuid::new_v4();

    let mut t = pool.begin().await.unwrap();
    let user = tx::users::create_with_id(&mut t, id, &new_user(&email))
        .await
        .unwrap();
    assert_eq!(user.id, id);
    assert!(tx::users::set_verified(&mut t, id, true).await.unwrap());
    // A host statement in the same transaction.
    let (echo,): (Uuid,) = sqlx::query_as("SELECT id FROM identity_users WHERE id = $1")
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .unwrap();
    assert_eq!(echo, id);
    t.commit().await.unwrap();

    let mut c = pool.acquire().await.unwrap();
    assert_eq!(
        tx::users::is_verified(&mut c, id).await.unwrap(),
        Some(true)
    );
    tx::users::delete(&mut c, id).await.unwrap();
}

#[tokio::test]
async fn duplicate_email_maps_to_domain_error_inside_a_transaction() {
    let pool = pool().await;
    let email = unique_email("tx-dup");

    let mut t = pool.begin().await.unwrap();
    tx::users::create(&mut t, &new_user(&email)).await.unwrap();
    let err = tx::users::create(&mut t, &new_user(&email.to_uppercase()))
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::EmailAlreadyTaken), "{err:?}");
    t.rollback().await.unwrap();
}

#[tokio::test]
async fn revocation_and_session_end_commit_atomically() {
    let pool = pool().await;
    let email = unique_email("tx-revoke");

    let mut c = pool.acquire().await.unwrap();
    let user = tx::users::create(&mut c, &new_user(&email)).await.unwrap();
    drop(c);

    let mut t = pool.begin().await.unwrap();
    assert!(tx::users::deactivate(&mut t, user.id).await.unwrap());
    let ended = tx::sessions::end_all_for_user(
        &mut t,
        user.id,
        z_auth::security::session::SessionStatus::Revoked,
        RevocationReason::AccountDisabled,
    )
    .await
    .unwrap();
    assert!(ended.is_empty());
    let revocation = tx::revocations::insert(
        &mut t,
        &NewRevocation {
            scope: RevocationScope::User(user.id),
            reason: RevocationReason::AccountDisabled,
            origin_node: Uuid::new_v4(),
            ttl: std::time::Duration::from_secs(330),
        },
    )
    .await
    .unwrap();
    t.rollback().await.unwrap();

    let mut c = pool.acquire().await.unwrap();
    assert_eq!(
        tx::users::is_active(&mut c, user.id).await.unwrap(),
        Some(true)
    );
    let active = tx::revocations::list_active(&mut c).await.unwrap();
    assert!(active.iter().all(|r| r.id != revocation.id));
    tx::users::delete(&mut c, user.id).await.unwrap();
}
