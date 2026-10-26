//! The organization catalog and users' memberships, against in-memory
//! adapters.
//!
//! Invariants checked here:
//! - organization names are unique case-insensitively, department names
//!   within their organization;
//! - a membership's department belongs to its organization;
//! - a user holds an `(organization, department)` pair once, the first
//!   membership becomes primary and at most one is;
//! - deleting a department keeps its members in the organization;
//!   deleting an organization or a user removes the memberships.

// The facade's defaults need the `argon2` and `crypto` features.
#![cfg(all(feature = "argon2", feature = "crypto"))]

mod support;

use uuid::Uuid;
use z_auth::conf::constants::MAX_MEMBERSHIPS_PER_USER;
use z_auth::controller::api::dto::{MembershipRequest, OrganizationRequest};
use z_auth::controller::api::handlers;
use z_auth::prelude::*;

use crate::support::{
    IP_A, Server, make_auth, profile_server, register, register_and_login, test_config,
};

fn org(name: &str) -> OrganizationInput {
    OrganizationInput {
        name: name.into(),
        description: None,
    }
}

fn dept(name: &str) -> DepartmentInput {
    DepartmentInput {
        name: name.into(),
        description: None,
    }
}

fn member_of(organization_id: Uuid, department_id: Option<Uuid>) -> MembershipInput {
    MembershipInput {
        organization_id,
        department_id,
        title: None,
        is_primary: false,
    }
}

async fn user(s: &Server, email: &str) -> Uuid {
    register(&s.auth, email).await.id
}

// ── Catalog ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_organization_catalog() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();

    let acme = orgs
        .create(OrganizationInput {
            name: "  Acme Corp ".into(),
            description: Some("  ".into()),
        })
        .await
        .unwrap();
    assert_eq!(acme.name, "Acme Corp");
    assert_eq!(acme.description, None);
    orgs.create(org("Globex")).await.unwrap();

    assert!(matches!(
        orgs.create(org("ACME CORP")).await,
        Err(AuthError::OrganizationAlreadyExists)
    ));
    assert!(matches!(
        orgs.create(org("   ")).await,
        Err(AuthError::InvalidProfileData(_))
    ));

    let page = orgs
        .list(OrganizationQuery {
            search: Some(" acm ".into()),
            ..OrganizationQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].id, acme.id);

    let page = orgs
        .list(OrganizationQuery {
            limit: 1,
            ..OrganizationQuery::default()
        })
        .await
        .unwrap();
    assert_eq!((page.items.len(), page.total), (1, 2));
    assert_eq!(page.items[0].name, "Acme Corp", "ordered by name");

    let renamed = orgs
        .update(
            acme.id,
            OrganizationInput {
                name: "Acme Inc".into(),
                description: Some("Rockets".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(renamed.name, "Acme Inc");
    assert!(matches!(
        orgs.update(acme.id, org("globex")).await,
        Err(AuthError::OrganizationAlreadyExists)
    ));
    assert!(matches!(
        orgs.update(Uuid::new_v4(), org("Nobody")).await,
        Err(AuthError::OrganizationNotFound)
    ));
}

#[tokio::test]
async fn test_departments() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let acme = orgs.create(org("Acme")).await.unwrap();
    let globex = orgs.create(org("Globex")).await.unwrap();

    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();
    orgs.create_department(acme.id, dept("Engineering"))
        .await
        .unwrap();
    // Department names are unique per organization only.
    orgs.create_department(globex.id, dept("Sales"))
        .await
        .unwrap();
    assert!(matches!(
        orgs.create_department(acme.id, dept("sales")).await,
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
    assert_eq!(renamed.organization_id, acme.id);
    assert!(matches!(
        orgs.update_department(sales.id, dept("engineering")).await,
        Err(AuthError::DepartmentAlreadyExists)
    ));
    assert!(matches!(
        orgs.departments(Uuid::new_v4()).await,
        Err(AuthError::OrganizationNotFound)
    ));
}

// ── Memberships ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_memberships() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let alice = user(&s, "alice@example.com").await;
    let acme = orgs.create(org("Acme")).await.unwrap();
    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();
    let globex = orgs.create(org("Globex")).await.unwrap();

    let first = orgs
        .add_membership(
            alice,
            MembershipInput {
                title: Some(" Account Manager ".into()),
                ..member_of(acme.id, Some(sales.id))
            },
        )
        .await
        .unwrap();
    assert!(first.is_primary, "the first membership becomes primary");
    assert_eq!(first.organization.name, "Acme");
    assert_eq!(first.department.as_ref().unwrap().name, "Sales");
    assert_eq!(first.title.as_deref(), Some("Account Manager"));

    // The same organization without a department is another membership…
    let whole = orgs
        .add_membership(alice, member_of(acme.id, None))
        .await
        .unwrap();
    assert!(!whole.is_primary);
    // …but each pair is held once.
    assert!(matches!(
        orgs.add_membership(alice, member_of(acme.id, Some(sales.id)))
            .await,
        Err(AuthError::MembershipAlreadyExists)
    ));
    assert!(matches!(
        orgs.add_membership(alice, member_of(acme.id, None)).await,
        Err(AuthError::MembershipAlreadyExists)
    ));

    // A department of another organization is refused.
    assert!(matches!(
        orgs.add_membership(alice, member_of(globex.id, Some(sales.id)))
            .await,
        Err(AuthError::DepartmentNotFound)
    ));
    assert!(matches!(
        orgs.add_membership(alice, member_of(Uuid::new_v4(), None))
            .await,
        Err(AuthError::OrganizationNotFound)
    ));

    let moved = orgs
        .add_membership(
            alice,
            MembershipInput {
                is_primary: true,
                ..member_of(globex.id, None)
            },
        )
        .await
        .unwrap();
    let memberships = orgs.memberships(alice).await.unwrap();
    assert_eq!(memberships[0].id, moved.id, "the primary is listed first");
    assert_eq!(memberships.iter().filter(|m| m.is_primary).count(), 1);

    orgs.set_primary_membership(alice, first.id).await.unwrap();
    let updated = orgs
        .update_membership(
            alice,
            first.id,
            MembershipInput {
                title: Some("Director".into()),
                ..member_of(acme.id, None)
            },
        )
        .await;
    assert!(
        matches!(updated, Err(AuthError::MembershipAlreadyExists)),
        "acme without a department is already held"
    );
    let updated = orgs
        .update_membership(
            alice,
            first.id,
            MembershipInput {
                title: Some("Director".into()),
                ..member_of(acme.id, Some(sales.id))
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.title.as_deref(), Some("Director"));
    assert!(updated.is_primary, "is_primary = false never demotes");

    let bob = user(&s, "bob@example.com").await;
    assert!(matches!(
        orgs.delete_membership(bob, first.id).await,
        Err(AuthError::MembershipNotFound)
    ));
    assert!(matches!(
        orgs.set_primary_membership(bob, first.id).await,
        Err(AuthError::MembershipNotFound)
    ));
    orgs.delete_membership(alice, whole.id).await.unwrap();
    assert_eq!(orgs.memberships(alice).await.unwrap().len(), 2);
}

#[tokio::test]
async fn test_members_listing() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let acme = orgs.create(org("Acme")).await.unwrap();
    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();
    let alice = user(&s, "alice@example.com").await;
    let bob = user(&s, "bob@example.com").await;
    let carol = user(&s, "carol@example.com").await;
    orgs.add_membership(alice, member_of(acme.id, Some(sales.id)))
        .await
        .unwrap();
    orgs.add_membership(bob, member_of(acme.id, None))
        .await
        .unwrap();
    orgs.add_membership(carol, member_of(acme.id, Some(sales.id)))
        .await
        .unwrap();

    let all = orgs.members(acme.id, MemberQuery::default()).await.unwrap();
    assert_eq!(all.total, 3);
    let users: Vec<_> = all.items.iter().map(|m| m.user_id).collect();
    assert_eq!(users, [alice, bob, carol], "oldest first");

    let in_sales = orgs
        .members(
            acme.id,
            MemberQuery {
                department_id: Some(sales.id),
                limit: 1,
                offset: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(in_sales.total, 2);
    assert_eq!(in_sales.items[0].user_id, carol);
}

#[tokio::test]
async fn test_membership_limit() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let alice = user(&s, "alice@example.com").await;
    for i in 0..MAX_MEMBERSHIPS_PER_USER {
        let o = orgs.create(org(&format!("Org {i}"))).await.unwrap();
        orgs.add_membership(alice, member_of(o.id, None))
            .await
            .unwrap();
    }
    let extra = orgs.create(org("One more")).await.unwrap();
    assert!(matches!(
        orgs.add_membership(alice, member_of(extra.id, None)).await,
        Err(AuthError::ProfileLimitReached(_))
    ));
}

// ── Deletion ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_deleting_a_department_keeps_its_members() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let alice = user(&s, "alice@example.com").await;
    let acme = orgs.create(org("Acme")).await.unwrap();
    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();
    let m = orgs
        .add_membership(alice, member_of(acme.id, Some(sales.id)))
        .await
        .unwrap();

    orgs.delete_department(sales.id).await.unwrap();

    let memberships = orgs.memberships(alice).await.unwrap();
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].id, m.id);
    assert_eq!(memberships[0].department, None);
    assert!(matches!(
        orgs.delete_department(sales.id).await,
        Err(AuthError::DepartmentNotFound)
    ));
}

#[tokio::test]
async fn test_deleting_an_organization_removes_memberships() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let alice = user(&s, "alice@example.com").await;
    let acme = orgs.create(org("Acme")).await.unwrap();
    let sales = orgs
        .create_department(acme.id, dept("Sales"))
        .await
        .unwrap();
    orgs.add_membership(alice, member_of(acme.id, Some(sales.id)))
        .await
        .unwrap();

    orgs.delete(acme.id).await.unwrap();

    assert!(orgs.memberships(alice).await.unwrap().is_empty());
    assert_eq!(orgs.find_department(sales.id).await.unwrap(), None);
    assert!(matches!(
        orgs.delete(acme.id).await,
        Err(AuthError::OrganizationNotFound)
    ));
}

#[tokio::test]
async fn test_deleting_the_user_removes_memberships() {
    let s = profile_server(test_config());
    let orgs = s.auth.organizations();
    let alice = user(&s, "alice@example.com").await;
    let acme = orgs.create(org("Acme")).await.unwrap();
    orgs.add_membership(alice, member_of(acme.id, None))
        .await
        .unwrap();

    s.auth.users().delete(alice).await.unwrap();

    let page = orgs.members(acme.id, MemberQuery::default()).await.unwrap();
    assert_eq!(page.total, 0);
}

// ── Wiring & API ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_without_the_store_calls_fail_with_config() {
    let auth = make_auth();
    assert!(matches!(
        auth.organizations()
            .list(OrganizationQuery::default())
            .await,
        Err(AuthError::Config(_))
    ));
}

#[tokio::test]
async fn test_handlers() {
    let s = profile_server(test_config());
    let (alice, pair) = register_and_login(&s.auth, "alice@example.com", IP_A).await;
    let bearer = pair.access_token.as_str();
    let (auth, orgs) = (s.auth.authentication(), s.auth.organizations());

    let acme = handlers::create_organization(
        orgs,
        OrganizationRequest {
            name: "Acme".into(),
            description: None,
        },
    )
    .await
    .unwrap();
    let err = handlers::create_organization(
        orgs,
        OrganizationRequest {
            name: "acme".into(),
            description: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!((err.status, err.code), (409, "organization_already_exists"));

    let joined = handlers::add_membership(
        auth,
        orgs,
        bearer,
        MembershipRequest {
            organization_id: acme.id,
            department_id: None,
            title: Some("Engineer".into()),
            is_primary: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(joined.user_id, alice.id);
    assert_eq!(joined.organization.name, "Acme");

    let mine = handlers::memberships(auth, orgs, bearer).await.unwrap();
    assert_eq!(mine.len(), 1);

    let members = handlers::organization_members(orgs, acme.id, MemberQuery::default())
        .await
        .unwrap();
    assert_eq!(members.total, 1);

    let err = handlers::organization(orgs, Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!((err.status, err.code), (404, "organization_not_found"));
}
