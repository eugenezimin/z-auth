//! Integration tests — users' phones and addresses, the organization
//! catalog and memberships against Postgres (feature `profile`).
//!
//! Run with:
//!   cargo test -p z-auth-postgres --features profile --test profile

mod helpers;

use std::sync::Arc;

use uuid::Uuid;
use z_auth::AuthError;
use z_auth::ZAuth;
use z_auth::conf::config::RawConfig;
use z_auth::identity::organization::{
    DepartmentInput, MemberQuery, MembershipInput, NewMembership, OrganizationInput,
    OrganizationQuery, OrganizationRepository,
};
use z_auth::identity::profile::{
    AddressInput, NewAddress, NewPhone, PhoneInput, ProfileRepository,
};
use z_auth::identity::user::NewUser;
use z_auth::identity::user::RegisterUser;
use z_auth::security::token::keys::{generate_refresh_secret, generate_signing_key};
use z_auth_postgres::{
    PgOrganizationRepository, PgPermissionRepository, PgProfileRepository, repositories,
    run_profile_migrations, tx,
};

use crate::helpers::{cleanup_user_by_id, create_test_user, pool, unique_email, unique_name};

const PASSWORD: &str = "S3cur3P@ssw0rd!";

struct Setup {
    auth: ZAuth,
    profiles: Arc<PgProfileRepository>,
    organizations: Arc<PgOrganizationRepository>,
}

async fn setup() -> Setup {
    let pool = pool().await;
    run_profile_migrations(&pool)
        .await
        .expect("profile migrations");
    let profiles = Arc::new(PgProfileRepository::new(pool.clone()));
    let organizations = Arc::new(PgOrganizationRepository::new(pool.clone()));
    let keys = generate_signing_key().unwrap();
    let config = RawConfig::default()
        .jwt_signing_key(keys.signing_key)
        .jwt_issuer("z-auth-test")
        .refresh_secret(generate_refresh_secret().unwrap())
        .build()
        .unwrap();
    let auth = ZAuth::builder(config, repositories(&pool))
        .permissions(Arc::new(PgPermissionRepository::new(pool.clone())))
        .profiles(profiles.clone())
        .organizations(organizations.clone())
        .build()
        .unwrap();
    Setup {
        auth,
        profiles,
        organizations,
    }
}

async fn user(s: &Setup, prefix: &str) -> Uuid {
    create_test_user(
        &s.auth,
        RegisterUser {
            email: unique_email(prefix),
            password: PASSWORD.into(),
            username: None,
            first_name: None,
            last_name: None,
        },
    )
    .await
    .id
}

fn phone(label: &str, number: &str) -> PhoneInput {
    PhoneInput {
        label: label.into(),
        number: number.into(),
        is_primary: false,
    }
}

fn new_phone(number: &str, is_primary: bool) -> NewPhone {
    NewPhone {
        label: "work".into(),
        number: number.into(),
        is_primary,
    }
}

fn address(label: &str) -> AddressInput {
    AddressInput {
        label: label.into(),
        line1: "1 Main St".into(),
        line2: Some("Apt 4".into()),
        locality: Some("Springfield".into()),
        region: Some("IL".into()),
        postal_code: Some("62701".into()),
        country_code: "us".into(),
        is_primary: false,
    }
}

fn org(name: &str) -> OrganizationInput {
    OrganizationInput {
        name: name.into(),
        description: Some("test organization".into()),
    }
}

fn dept(name: &str) -> DepartmentInput {
    DepartmentInput {
        name: name.into(),
        description: None,
    }
}

fn membership(organization_id: Uuid, department_id: Option<Uuid>) -> NewMembership {
    NewMembership {
        organization_id,
        department_id,
        title: None,
        is_primary: false,
    }
}

// ── Phones ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_phones_round_trip() {
    let s = setup().await;
    let alice = user(&s, "phones").await;
    let profiles = s.auth.profiles();

    let work = profiles
        .add_phone(alice, phone(" Work ", "+1 (555) 123-4567"))
        .await
        .unwrap();
    assert_eq!(
        (work.label.as_str(), work.number.as_str()),
        ("Work", "+15551234567")
    );
    assert!(work.is_primary);
    let home = profiles
        .add_phone(alice, phone("home", "+49 30 1234567"))
        .await
        .unwrap();
    assert!(!home.is_primary);

    // The unique constraint maps to its domain error.
    assert!(matches!(
        profiles
            .add_phone(alice, phone("dup", "+15551234567"))
            .await,
        Err(AuthError::PhoneAlreadyAdded)
    ));
    assert!(matches!(
        profiles
            .replace_phone(alice, home.id, phone("home", "+15551234567"))
            .await,
        Err(AuthError::PhoneAlreadyAdded)
    ));

    let replaced = profiles
        .replace_phone(
            alice,
            home.id,
            PhoneInput {
                is_primary: true,
                ..phone("Home", "+493099999999")
            },
        )
        .await
        .unwrap();
    assert!(replaced.is_primary);
    assert!(replaced.updated_at >= replaced.created_at);

    let phones = profiles.phones(alice).await.unwrap();
    assert_eq!(phones.len(), 2);
    assert_eq!(phones[0].id, home.id, "primary first");
    assert!(!phones[1].is_primary);

    // A missing phone rolls back the clearing of the primary.
    assert!(matches!(
        profiles
            .replace_phone(
                alice,
                Uuid::new_v4(),
                PhoneInput {
                    is_primary: true,
                    ..phone("x", "+15550000000")
                }
            )
            .await,
        Err(AuthError::PhoneNotFound)
    ));
    assert!(profiles.phones(alice).await.unwrap()[0].is_primary);

    profiles.set_primary_phone(alice, work.id).await.unwrap();
    assert_eq!(profiles.phones(alice).await.unwrap()[0].id, work.id);
    profiles.delete_phone(alice, work.id).await.unwrap();
    assert_eq!(profiles.phones(alice).await.unwrap().len(), 1);

    cleanup_user_by_id(&s.auth, alice).await.unwrap();
}

#[tokio::test]
async fn test_phones_are_scoped_to_their_user() {
    let s = setup().await;
    let alice = user(&s, "scope_a").await;
    let bob = user(&s, "scope_b").await;

    let alices = s
        .profiles
        .add_phone(alice, new_phone("+15551234567", true))
        .await
        .unwrap();
    s.profiles
        .add_phone(bob, new_phone("+15551234567", true))
        .await
        .expect("two users may share a number");

    assert!(
        s.profiles
            .replace_phone(bob, alices.id, new_phone("+15550000000", false))
            .await
            .unwrap()
            .is_none()
    );
    assert!(!s.profiles.set_primary_phone(bob, alices.id).await.unwrap());
    assert!(!s.profiles.delete_phone(bob, alices.id).await.unwrap());
    assert_eq!(s.profiles.list_phones(alice).await.unwrap(), [alices]);

    cleanup_user_by_id(&s.auth, alice).await.unwrap();
    cleanup_user_by_id(&s.auth, bob).await.unwrap();
}

#[tokio::test]
async fn test_unknown_user() {
    let s = setup().await;
    assert!(matches!(
        s.profiles
            .add_phone(Uuid::new_v4(), new_phone("+15551234567", false))
            .await,
        Err(AuthError::UserNotFound)
    ));
    assert!(
        !s.profiles
            .set_primary_phone(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap()
    );
}

/// Primary changes for one user serialize on the user's row: concurrent
/// ones never trip the partial unique index, and one primary remains.
#[tokio::test]
async fn test_concurrent_primary_changes() {
    let s = setup().await;
    let alice = user(&s, "concurrent").await;

    // Concurrent first inserts, all asking to be primary.
    let inserts = (0..10).map(|i| {
        let repo = s.profiles.clone();
        tokio::spawn(async move {
            repo.add_phone(alice, new_phone(&format!("+1555000{i:04}"), true))
                .await
        })
    });
    for task in inserts.collect::<Vec<_>>() {
        task.await.unwrap().expect("no unique violation");
    }
    let phones = s.profiles.list_phones(alice).await.unwrap();
    assert_eq!(phones.len(), 10);
    assert_eq!(phones.iter().filter(|p| p.is_primary).count(), 1);

    let flips = (0..40).map(|i| {
        let repo = s.profiles.clone();
        let id = phones[i % phones.len()].id;
        tokio::spawn(async move { repo.set_primary_phone(alice, id).await })
    });
    for task in flips.collect::<Vec<_>>() {
        assert!(task.await.unwrap().expect("no unique violation"));
    }
    let phones = s.profiles.list_phones(alice).await.unwrap();
    assert_eq!(phones.iter().filter(|p| p.is_primary).count(), 1);

    cleanup_user_by_id(&s.auth, alice).await.unwrap();
}

// ── Addresses ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_addresses_round_trip() {
    let s = setup().await;
    let alice = user(&s, "addresses").await;
    let profiles = s.auth.profiles();

    let home = profiles.add_address(alice, address("home")).await.unwrap();
    assert!(home.is_primary);
    assert_eq!(home.country_code, "US");
    assert_eq!(home.line2.as_deref(), Some("Apt 4"));

    let work = profiles
        .replace_address(
            alice,
            home.id,
            AddressInput {
                line2: None,
                region: None,
                country_code: "de".into(),
                ..address("work")
            },
        )
        .await
        .unwrap();
    assert_eq!(work.line2, None);
    assert_eq!(work.region, None);
    assert_eq!(work.country_code, "DE");
    assert!(work.is_primary, "is_primary = false keeps the flag");

    let other = profiles.add_address(alice, address("other")).await.unwrap();
    profiles.set_primary_address(alice, other.id).await.unwrap();
    let addresses = profiles.addresses(alice).await.unwrap();
    assert_eq!(addresses[0].id, other.id);
    assert!(!addresses[1].is_primary);

    // The database's own checks back the service's validation.
    let bad = NewAddress {
        label: "bad".into(),
        line1: "1 Main St".into(),
        line2: None,
        locality: None,
        region: None,
        postal_code: None,
        country_code: "us".into(),
        is_primary: false,
    };
    assert!(matches!(
        s.profiles.add_address(alice, bad).await,
        Err(AuthError::Storage(_))
    ));

    profiles.delete_address(alice, other.id).await.unwrap();
    assert!(matches!(
        profiles.delete_address(alice, other.id).await,
        Err(AuthError::AddressNotFound)
    ));

    cleanup_user_by_id(&s.auth, alice).await.unwrap();
}

#[tokio::test]
async fn test_deleting_the_user_deletes_the_profile() {
    let s = setup().await;
    let alice = user(&s, "cascade").await;
    let acme = s
        .auth
        .organizations()
        .create(org(&unique_name("Cascade Co")))
        .await
        .unwrap();
    s.auth
        .profiles()
        .add_phone(alice, phone("work", "+15551234567"))
        .await
        .unwrap();
    s.auth
        .profiles()
        .add_address(alice, address("home"))
        .await
        .unwrap();
    s.auth
        .organizations()
        .add_membership(
            alice,
            MembershipInput {
                organization_id: acme.id,
                ..MembershipInput::default()
            },
        )
        .await
        .unwrap();

    cleanup_user_by_id(&s.auth, alice).await.unwrap();

    assert!(s.profiles.list_phones(alice).await.unwrap().is_empty());
    assert!(s.profiles.list_addresses(alice).await.unwrap().is_empty());
    assert!(
        s.organizations
            .list_memberships(alice)
            .await
            .unwrap()
            .is_empty()
    );
    s.auth.organizations().delete(acme.id).await.unwrap();
}

// ── Organizations ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_organization_catalog() {
    let s = setup().await;
    let orgs = s.auth.organizations();
    let name = unique_name("Acme");

    let acme = orgs.create(org(&name)).await.unwrap();
    assert_eq!(acme.description.as_deref(), Some("test organization"));
    assert!(matches!(
        orgs.create(org(&name.to_uppercase())).await,
        Err(AuthError::OrganizationAlreadyExists)
    ));

    let page = orgs
        .list(OrganizationQuery {
            search: Some(name.to_uppercase()),
            ..OrganizationQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, acme.id);

    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();
    orgs.create_department(acme.id, dept("Engineering"))
        .await
        .unwrap();
    assert!(matches!(
        orgs.create_department(acme.id, dept("SALES")).await,
        Err(AuthError::DepartmentAlreadyExists)
    ));
    assert!(matches!(
        orgs.create_department(Uuid::new_v4(), dept("Sales")).await,
        Err(AuthError::OrganizationNotFound)
    ));
    let names: Vec<_> = orgs
        .departments(acme.id)
        .await
        .unwrap()
        .into_iter()
        .map(|d| d.name)
        .collect();
    assert_eq!(names, ["Engineering", "Sales"]);

    let renamed = orgs
        .update_department(sales.id, dept("Sales & Marketing"))
        .await
        .unwrap();
    assert_eq!(renamed.name, "Sales & Marketing");
    let updated = orgs
        .update(
            acme.id,
            OrganizationInput {
                name: format!("{name} Inc"),
                description: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.description, None);

    orgs.delete(acme.id).await.unwrap();
    assert_eq!(orgs.find_department(sales.id).await.unwrap(), None);
}

#[tokio::test]
async fn test_memberships() {
    let s = setup().await;
    let orgs = s.auth.organizations();
    let alice = user(&s, "member_a").await;
    let bob = user(&s, "member_b").await;
    let acme = orgs.create(org(&unique_name("Acme"))).await.unwrap();
    let globex = orgs.create(org(&unique_name("Globex"))).await.unwrap();
    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();

    let first = s
        .organizations
        .add_membership(
            alice,
            NewMembership {
                title: Some("Account Manager".into()),
                is_primary: true,
                ..membership(acme.id, Some(sales.id))
            },
        )
        .await
        .unwrap();
    assert_eq!(first.organization.id, acme.id);
    assert_eq!(first.department.as_ref().unwrap().name, "Sales");
    assert!(first.is_primary);

    let whole = s
        .organizations
        .add_membership(alice, membership(acme.id, None))
        .await
        .unwrap();
    assert_eq!(whole.department, None);
    // NULLS NOT DISTINCT: "no department" is held once as well.
    assert!(matches!(
        s.organizations
            .add_membership(alice, membership(acme.id, None))
            .await,
        Err(AuthError::MembershipAlreadyExists)
    ));
    assert!(matches!(
        s.organizations
            .add_membership(alice, membership(acme.id, Some(sales.id)))
            .await,
        Err(AuthError::MembershipAlreadyExists)
    ));
    // The composite foreign key refuses another organization's department,
    // even past the service's own check.
    assert!(matches!(
        s.organizations
            .add_membership(alice, membership(globex.id, Some(sales.id)))
            .await,
        Err(AuthError::DepartmentNotFound)
    ));
    assert!(matches!(
        s.organizations
            .add_membership(alice, membership(Uuid::new_v4(), None))
            .await,
        Err(AuthError::OrganizationNotFound)
    ));

    let moved = s
        .organizations
        .update_membership(
            alice,
            whole.id,
            NewMembership {
                title: Some("Consultant".into()),
                is_primary: true,
                ..membership(globex.id, None)
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.organization.id, globex.id);
    let memberships = s.organizations.list_memberships(alice).await.unwrap();
    assert_eq!(memberships[0].id, whole.id, "primary first");
    assert_eq!(memberships.iter().filter(|m| m.is_primary).count(), 1);

    // Scoped to the user.
    assert!(
        s.organizations
            .update_membership(bob, first.id, membership(acme.id, None))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        !s.organizations
            .set_primary_membership(bob, first.id)
            .await
            .unwrap()
    );
    assert!(
        !s.organizations
            .delete_membership(bob, first.id)
            .await
            .unwrap()
    );

    s.organizations
        .add_membership(bob, membership(acme.id, None))
        .await
        .unwrap();
    let all = orgs.members(acme.id, MemberQuery::default()).await.unwrap();
    assert_eq!(all.total, 2);
    assert_eq!(all.items[0].user_id, alice, "oldest first");
    let in_sales = orgs
        .members(
            acme.id,
            MemberQuery {
                department_id: Some(sales.id),
                ..MemberQuery::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(in_sales.total, 1);
    assert_eq!(in_sales.items[0].id, first.id);
    let paged = orgs
        .members(
            acme.id,
            MemberQuery {
                limit: 1,
                offset: 1,
                ..MemberQuery::default()
            },
        )
        .await
        .unwrap();
    assert_eq!((paged.items.len(), paged.total), (1, 2));
    assert_eq!(paged.items[0].user_id, bob);

    // Deleting the department keeps the membership, without department.
    orgs.delete_department(sales.id).await.unwrap();
    let kept = s.organizations.list_memberships(alice).await.unwrap();
    let kept = kept.iter().find(|m| m.id == first.id).unwrap();
    assert_eq!(kept.organization.id, acme.id);
    assert_eq!(kept.department, None);

    // Deleting the organization deletes its memberships.
    orgs.delete(acme.id).await.unwrap();
    let left = s.organizations.list_memberships(alice).await.unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].organization.id, globex.id);

    orgs.delete(globex.id).await.unwrap();
    cleanup_user_by_id(&s.auth, alice).await.unwrap();
    cleanup_user_by_id(&s.auth, bob).await.unwrap();
}

// ── In the host's transaction (tx::profiles, tx::organizations) ───────────────

fn new_user(email: &str) -> NewUser {
    NewUser {
        email: email.to_string(),
        password_hash: None,
        username: None,
        first_name: None,
        last_name: None,
        is_verified: false,
    }
}

#[tokio::test]
async fn test_rolled_back_transaction_leaves_no_profile() {
    let s = setup().await;
    let pool = pool().await;
    let email = unique_email("tx_profile_rollback");
    let org_name = unique_name("Rollback Co");

    let mut t = pool.begin().await.unwrap();
    let user = tx::users::create(&mut t, &new_user(&email)).await.unwrap();
    let phone = tx::profiles::add_phone(&mut t, user.id, &new_phone("+15551234567", true))
        .await
        .unwrap();
    let org = tx::organizations::create_organization(&mut t, &org(&org_name))
        .await
        .unwrap();
    tx::organizations::add_membership(&mut t, user.id, &membership(org.id, None))
        .await
        .unwrap();
    // Visible inside the transaction.
    assert_eq!(
        tx::profiles::list_phones(&mut t, user.id).await.unwrap(),
        [phone]
    );
    t.rollback().await.unwrap();

    let mut c = pool.acquire().await.unwrap();
    assert!(
        tx::users::find_by_email(&mut c, &email)
            .await
            .unwrap()
            .is_none()
    );
    assert!(s.profiles.list_phones(user.id).await.unwrap().is_empty());
    assert!(
        tx::organizations::find_organization(&mut c, org.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_committed_transaction_includes_host_statements() {
    let s = setup().await;
    let pool = pool().await;
    let alice = user(&s, "tx_profile_commit").await;

    let mut t = pool.begin().await.unwrap();
    let home = tx::profiles::add_address(
        &mut t,
        alice,
        &NewAddress {
            label: "home".into(),
            line1: "1 Main St".into(),
            line2: None,
            locality: None,
            region: None,
            postal_code: None,
            country_code: "US".into(),
            is_primary: true,
        },
    )
    .await
    .unwrap();
    // A host statement in the same transaction.
    let (echo,): (Uuid,) =
        sqlx::query_as("SELECT user_id FROM identity_user_addresses WHERE id = $1")
            .bind(home.id)
            .fetch_one(&mut *t)
            .await
            .unwrap();
    assert_eq!(echo, alice);
    t.commit().await.unwrap();

    assert_eq!(s.profiles.list_addresses(alice).await.unwrap(), [home]);
    cleanup_user_by_id(&s.auth, alice).await.unwrap();
}

#[tokio::test]
async fn test_not_found_changes_nothing_inside_a_transaction() {
    let s = setup().await;
    let pool = pool().await;
    let alice = user(&s, "tx_profile_noop").await;
    let primary = s
        .profiles
        .add_phone(alice, new_phone("+15551234567", true))
        .await
        .unwrap();

    let mut t = pool.begin().await.unwrap();
    // Neither call finds its phone, so the primary must survive even
    // though the transaction commits.
    assert!(
        tx::profiles::replace_phone(
            &mut t,
            alice,
            Uuid::new_v4(),
            &new_phone("+15550000000", true)
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(
        !tx::profiles::set_primary_phone(&mut t, alice, Uuid::new_v4())
            .await
            .unwrap()
    );
    t.commit().await.unwrap();
    assert_eq!(
        s.profiles.list_phones(alice).await.unwrap(),
        std::slice::from_ref(&primary)
    );

    let mut t = pool.begin().await.unwrap();
    // Unique violations still map to domain errors.
    let err = tx::profiles::add_phone(&mut t, alice, &new_phone("+15551234567", false))
        .await
        .unwrap_err();
    assert!(matches!(err, AuthError::PhoneAlreadyAdded), "{err:?}");
    t.rollback().await.unwrap();

    assert_eq!(s.profiles.list_phones(alice).await.unwrap(), [primary]);
    cleanup_user_by_id(&s.auth, alice).await.unwrap();
}
