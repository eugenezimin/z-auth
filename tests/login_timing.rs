//! Login spends one password verification whether or not the email is
//! registered, so response time does not reveal registered emails.

#![cfg(all(feature = "argon2", feature = "crypto"))]

mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use z_auth::identity::credentials::{Argon2Hasher, PasswordHasher};
use z_auth::prelude::*;

use crate::support::{
    IP_A, InMemoryDb, MockClock, credentials, ctx, register_request, repositories, test_config,
};

/// Argon2 that counts real and dummy verifications.
#[derive(Default)]
struct CountingHasher {
    verified: AtomicUsize,
    dummies: AtomicUsize,
}

impl PasswordHasher for CountingHasher {
    fn hash(&self, plaintext: &str) -> Result<String, AuthError> {
        Argon2Hasher.hash(plaintext)
    }
    fn verify(&self, plaintext: &str, stored_hash: &str) -> Result<bool, AuthError> {
        self.verified.fetch_add(1, Ordering::SeqCst);
        Argon2Hasher.verify(plaintext, stored_hash)
    }
    fn verify_dummy(&self, plaintext: &str) {
        self.dummies.fetch_add(1, Ordering::SeqCst);
        Argon2Hasher.verify_dummy(plaintext);
    }
}

fn auth_with(hasher: Arc<CountingHasher>) -> ZAuth {
    let clock = MockClock::new();
    let db = Arc::new(InMemoryDb::new(clock.clone()));
    ZAuth::builder(test_config(), repositories(&db))
        .permissions(db.clone())
        .clock(clock)
        .password_hasher(hasher)
        .build()
        .expect("ZAuth must build")
}

#[tokio::test]
async fn unknown_email_spends_a_dummy_verification() {
    let hasher = Arc::new(CountingHasher::default());
    let auth = auth_with(hasher.clone());

    let err = auth
        .authentication()
        .login(credentials("nobody@example.com"), ctx(IP_A))
        .await
        .unwrap_err();

    assert!(matches!(err, AuthError::InvalidCredentials));
    assert_eq!(hasher.dummies.load(Ordering::SeqCst), 1);
    assert_eq!(hasher.verified.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn known_email_with_wrong_password_spends_a_real_verification() {
    let hasher = Arc::new(CountingHasher::default());
    let auth = auth_with(hasher.clone());
    auth.users()
        .register(register_request("alice@example.com", None))
        .await
        .unwrap();

    let mut wrong = credentials("alice@example.com");
    wrong.password = "not-the-password-1A!".into();
    let err = auth
        .authentication()
        .login(wrong, ctx(IP_A))
        .await
        .unwrap_err();

    assert!(matches!(err, AuthError::InvalidCredentials));
    assert_eq!(hasher.dummies.load(Ordering::SeqCst), 0);
    assert_eq!(hasher.verified.load(Ordering::SeqCst), 1);
}

#[test]
fn argon2_dummy_verification_does_not_panic() {
    Argon2Hasher.verify_dummy("anything");
    Argon2Hasher.verify_dummy("");
}
