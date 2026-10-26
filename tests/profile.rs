//! Users' optional phone numbers and addresses, against in-memory adapters.
//!
//! Invariants checked here:
//! - inputs are validated and normalized (E.164 numbers, trimmed labels,
//!   uppercase country codes);
//! - a user's first item of a kind becomes primary, and at most one is;
//! - one user can never see or change another user's items;
//! - per-user limits hold;
//! - deleting the user deletes their profile;
//! - without the store every call is a configuration error.

// The facade's defaults need the `argon2` and `crypto` features.
#![cfg(all(feature = "argon2", feature = "crypto"))]

mod support;

use uuid::Uuid;
use z_auth::conf::constants::{MAX_ADDRESSES_PER_USER, MAX_PHONES_PER_USER};
use z_auth::controller::api::dto::{AddressRequest, PhoneRequest};
use z_auth::controller::api::handlers;
use z_auth::prelude::*;

use crate::support::{
    IP_A, Server, make_auth, profile_server, register, register_and_login, test_config,
};

fn phone(label: &str, number: &str) -> PhoneInput {
    PhoneInput {
        label: label.into(),
        number: number.into(),
        is_primary: false,
    }
}

fn address(label: &str, line1: &str) -> AddressInput {
    AddressInput {
        label: label.into(),
        line1: line1.into(),
        line2: None,
        locality: Some("Springfield".into()),
        region: None,
        postal_code: Some("12345".into()),
        country_code: "us".into(),
        is_primary: false,
    }
}

fn primaries(phones: &[Phone]) -> Vec<&str> {
    phones
        .iter()
        .filter(|p| p.is_primary)
        .map(|p| p.label.as_str())
        .collect()
}

async fn user(s: &Server, email: &str) -> Uuid {
    register(&s.auth, email).await.id
}

// ── Phones ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_phone_is_normalized_and_first_becomes_primary() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;

    let work = s
        .auth
        .profiles()
        .add_phone(alice, phone("  Work ", "+1 (555) 123-4567"))
        .await
        .unwrap();
    assert_eq!(work.label, "Work");
    assert_eq!(work.number, "+15551234567");
    assert!(work.is_primary, "the first phone becomes primary");

    let home = s
        .auth
        .profiles()
        .add_phone(alice, phone("home", "+49 30 1234567"))
        .await
        .unwrap();
    assert!(!home.is_primary);

    let phones = s.auth.profiles().phones(alice).await.unwrap();
    assert_eq!(phones.len(), 2);
    assert_eq!(primaries(&phones), ["Work"]);
}

#[tokio::test]
async fn test_phone_validation() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let profiles = s.auth.profiles();

    for bad in ["5551234567", "+0123456", "+1 555 CALL NOW"] {
        assert!(
            matches!(
                profiles.add_phone(alice, phone("work", bad)).await,
                Err(AuthError::InvalidPhoneNumber(_))
            ),
            "{bad}"
        );
    }
    assert!(matches!(
        profiles.add_phone(alice, phone("  ", "+15551234567")).await,
        Err(AuthError::InvalidProfileData(_))
    ));

    profiles
        .add_phone(alice, phone("work", "+15551234567"))
        .await
        .unwrap();
    assert!(matches!(
        profiles
            .add_phone(alice, phone("other", "+1 555 123 4567"))
            .await,
        Err(AuthError::PhoneAlreadyAdded)
    ));
}

#[tokio::test]
async fn test_primary_phone_moves_and_is_unique() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let profiles = s.auth.profiles();

    profiles
        .add_phone(alice, phone("work", "+15551230001"))
        .await
        .unwrap();
    let home = profiles
        .add_phone(alice, phone("home", "+15551230002"))
        .await
        .unwrap();
    let mobile = profiles
        .add_phone(
            alice,
            PhoneInput {
                is_primary: true,
                ..phone("mobile", "+15551230003")
            },
        )
        .await
        .unwrap();
    assert!(mobile.is_primary);
    assert_eq!(
        primaries(&profiles.phones(alice).await.unwrap()),
        ["mobile"]
    );

    profiles.set_primary_phone(alice, home.id).await.unwrap();
    let phones = profiles.phones(alice).await.unwrap();
    assert_eq!(primaries(&phones), ["home"]);
    assert_eq!(phones[0].id, home.id, "the primary is listed first");

    // Replacing with is_primary = false keeps the flag.
    let replaced = profiles
        .replace_phone(alice, home.id, phone("Home", "+15551230099"))
        .await
        .unwrap();
    assert!(replaced.is_primary);
    assert_eq!(replaced.number, "+15551230099");

    // Deleting the primary does not promote another phone.
    profiles.delete_phone(alice, home.id).await.unwrap();
    let phones = profiles.phones(alice).await.unwrap();
    assert_eq!(phones.len(), 2);
    assert!(primaries(&phones).is_empty());
}

#[tokio::test]
async fn test_phones_are_scoped_to_their_user() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let bob = user(&s, "bob@example.com").await;
    let profiles = s.auth.profiles();

    let alices = profiles
        .add_phone(alice, phone("work", "+15551234567"))
        .await
        .unwrap();
    // The same number may belong to two users.
    profiles
        .add_phone(bob, phone("work", "+15551234567"))
        .await
        .unwrap();

    assert!(matches!(
        profiles
            .replace_phone(bob, alices.id, phone("mine", "+15550000000"))
            .await,
        Err(AuthError::PhoneNotFound)
    ));
    assert!(matches!(
        profiles.set_primary_phone(bob, alices.id).await,
        Err(AuthError::PhoneNotFound)
    ));
    assert!(matches!(
        profiles.delete_phone(bob, alices.id).await,
        Err(AuthError::PhoneNotFound)
    ));
    assert_eq!(profiles.phones(alice).await.unwrap(), [alices]);
}

#[tokio::test]
async fn test_phone_limit() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let profiles = s.auth.profiles();

    for i in 0..MAX_PHONES_PER_USER {
        profiles
            .add_phone(alice, phone("work", &format!("+1555000{i:04}")))
            .await
            .unwrap();
    }
    assert!(matches!(
        profiles
            .add_phone(alice, phone("one more", "+15559999999"))
            .await,
        Err(AuthError::ProfileLimitReached(_))
    ));
}

#[tokio::test]
async fn test_unknown_user() {
    let s = profile_server(test_config());
    assert!(matches!(
        s.auth
            .profiles()
            .add_phone(Uuid::new_v4(), phone("work", "+15551234567"))
            .await,
        Err(AuthError::UserNotFound)
    ));
}

// ── Addresses ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_addresses() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let profiles = s.auth.profiles();

    let home = profiles
        .add_address(
            alice,
            AddressInput {
                line2: Some("   ".into()),
                ..address("home", " 1 Main St ")
            },
        )
        .await
        .unwrap();
    assert!(home.is_primary);
    assert_eq!(home.line1, "1 Main St");
    assert_eq!(home.line2, None);
    assert_eq!(home.country_code, "US");

    let work = profiles
        .add_address(alice, address("work", "2 Office Park"))
        .await
        .unwrap();
    assert!(!work.is_primary);

    // Replacing clears an optional field and can take the primary flag.
    let work = profiles
        .replace_address(
            alice,
            work.id,
            AddressInput {
                postal_code: None,
                is_primary: true,
                ..address("work", "3 Office Park")
            },
        )
        .await
        .unwrap();
    assert_eq!(work.line1, "3 Office Park");
    assert_eq!(work.postal_code, None);
    assert!(work.is_primary);

    let addresses = profiles.addresses(alice).await.unwrap();
    assert_eq!(addresses[0].id, work.id);
    assert!(!addresses[1].is_primary);

    assert!(matches!(
        profiles
            .add_address(
                alice,
                AddressInput {
                    country_code: "USA".into(),
                    ..address("x", "4 Elm St")
                }
            )
            .await,
        Err(AuthError::InvalidProfileData(_))
    ));
    assert!(matches!(
        profiles.add_address(alice, address("x", "  ")).await,
        Err(AuthError::InvalidProfileData(_))
    ));

    let bob = user(&s, "bob@example.com").await;
    assert!(matches!(
        profiles.delete_address(bob, home.id).await,
        Err(AuthError::AddressNotFound)
    ));
    profiles.delete_address(alice, home.id).await.unwrap();
    assert_eq!(profiles.addresses(alice).await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_address_limit() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let profiles = s.auth.profiles();
    for i in 0..MAX_ADDRESSES_PER_USER {
        profiles
            .add_address(alice, address("home", &format!("{i} Main St")))
            .await
            .unwrap();
    }
    assert!(matches!(
        profiles
            .add_address(alice, address("home", "99 Main St"))
            .await,
        Err(AuthError::ProfileLimitReached(_))
    ));
}

// ── Lifecycle & wiring ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_deleting_the_user_deletes_the_profile() {
    let s = profile_server(test_config());
    let alice = user(&s, "alice@example.com").await;
    let profiles = s.auth.profiles();
    profiles
        .add_phone(alice, phone("work", "+15551234567"))
        .await
        .unwrap();
    profiles
        .add_address(alice, address("home", "1 Main St"))
        .await
        .unwrap();

    s.auth.users().delete(alice).await.unwrap();

    assert!(profiles.phones(alice).await.unwrap().is_empty());
    assert!(profiles.addresses(alice).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_without_the_store_calls_fail_with_config() {
    let auth = make_auth();
    assert!(matches!(
        auth.profiles().phones(Uuid::new_v4()).await,
        Err(AuthError::Config(_))
    ));
}

#[tokio::test]
async fn test_me_handlers_act_for_the_bearer() {
    let s = profile_server(test_config());
    let (alice, pair) = register_and_login(&s.auth, "alice@example.com", IP_A).await;
    let bearer = pair.access_token.as_str();
    let (auth, profiles) = (s.auth.authentication(), s.auth.profiles());

    let added = handlers::add_phone(
        auth,
        profiles,
        bearer,
        PhoneRequest {
            label: "work".into(),
            number: "+1 555 123 4567".into(),
            is_primary: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(added.number, "+15551234567");
    assert!(added.is_primary);
    assert_eq!(profiles.phones(alice.id).await.unwrap()[0].id, added.id);

    handlers::add_address(
        auth,
        profiles,
        bearer,
        AddressRequest {
            label: "home".into(),
            line1: "1 Main St".into(),
            line2: None,
            locality: None,
            region: None,
            postal_code: None,
            country_code: "DE".into(),
            is_primary: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        handlers::addresses(auth, profiles, bearer)
            .await
            .unwrap()
            .len(),
        1
    );

    let err = handlers::delete_phone(auth, profiles, bearer, Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!((err.status, err.code), (404, "phone_not_found"));

    let err = handlers::phones(auth, profiles, "not-a-token")
        .await
        .unwrap_err();
    assert_eq!(err.status, 401);

    handlers::delete_phone(auth, profiles, bearer, added.id)
        .await
        .unwrap();
    assert!(
        handlers::phones(auth, profiles, bearer)
            .await
            .unwrap()
            .is_empty()
    );
}
