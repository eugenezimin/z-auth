//! Argon2id implementation of [`PasswordHasher`].
//!
//! Argon2id (memory-hard, recommended for password storage) produces a
//! self-describing string (e.g. `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`)
//! that encodes the algorithm, version, parameters, salt, and digest — no
//! separate salt column is required.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher as _, PasswordVerifier, phc::PasswordHash},
};

use std::sync::OnceLock;

use crate::identity::credentials::hasher::PasswordHasher;
use crate::utils::error::AuthError;

/// Argon2id with the crate's OWASP-recommended default parameters.
#[derive(Debug, Default, Clone, Copy)]
pub struct Argon2Hasher;

impl PasswordHasher for Argon2Hasher {
    /// Generates a fresh random salt from the OS on every call, so two calls
    /// with the same password produce different hashes.
    fn hash(&self, plaintext: &str) -> Result<String, AuthError> {
        Argon2::default()
            .hash_password(plaintext.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::HashingError(e.to_string()))
    }

    fn verify(&self, plaintext: &str, stored_hash: &str) -> Result<bool, AuthError> {
        let parsed = PasswordHash::new(stored_hash)
            .map_err(|e| AuthError::HashingError(format!("malformed stored hash: {e}")))?;

        match Argon2::default().verify_password(plaintext.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(e) => Err(AuthError::HashingError(e.to_string())),
        }
    }

    /// Verifies against a fixed hash made once with the same parameters, so
    /// it costs what a real verification costs.
    fn verify_dummy(&self, plaintext: &str) {
        static DUMMY: OnceLock<Option<String>> = OnceLock::new();
        if let Some(hash) = DUMMY.get_or_init(|| self.hash("z-auth timing equaliser").ok()) {
            let _ = self.verify(plaintext, hash);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Made by argon2 0.5.3 with `Argon2::default()`, the hasher z-auth used
    /// up to 0.7.0. Passwords stored back then must keep verifying.
    const HASH_FROM_ARGON2_0_5: &str = "$argon2id$v=19$m=19456,t=2,p=1$\
         tDH8UNGINvU8/fdZVIzBJQ$jrv8CNF4Eyti56g7uyOpra0aXnkIKdisxDab3ve2pBk";

    #[test]
    fn verifies_hashes_stored_by_earlier_releases() {
        let hasher = Argon2Hasher;
        assert!(matches!(
            hasher.verify("correct horse battery staple", HASH_FROM_ARGON2_0_5),
            Ok(true)
        ));
        assert!(matches!(
            hasher.verify("wrong horse battery staple", HASH_FROM_ARGON2_0_5),
            Ok(false)
        ));
    }

    #[test]
    fn hashes_in_the_same_format() {
        let hasher = Argon2Hasher;
        let hash = hasher.hash("pw").unwrap_or_default();
        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{hash}"
        );
        assert!(matches!(hasher.verify("pw", &hash), Ok(true)));
    }
}
