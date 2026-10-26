//! Organizations — a shared catalog of organizations and their departments,
//! and users' optional memberships in them.

pub mod model;
pub mod repository;
pub mod service;
pub mod service_impl;
pub mod validation;

pub use model::{
    DEFAULT_ORGANIZATION_PAGE, Department, DepartmentInput, DepartmentRef, MAX_ORGANIZATION_PAGE,
    MemberQuery, Membership, MembershipInput, MembershipPage, NewMembership, Organization,
    OrganizationInput, OrganizationPage, OrganizationQuery, OrganizationRef,
};
pub use repository::OrganizationRepository;
pub use service::OrganizationService;
pub use service_impl::OrganizationServiceImpl;
