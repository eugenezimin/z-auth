//! Organization service interface.
//!
//! Defines the [`OrganizationService`] trait — the organization catalog and
//! users' memberships.  The default implementation is
//! [`OrganizationServiceImpl`](crate::identity::organization::OrganizationServiceImpl).

use async_trait::async_trait;
use uuid::Uuid;

use crate::identity::organization::model::{
    Department, DepartmentInput, MemberQuery, Membership, MembershipInput, MembershipPage,
    Organization, OrganizationInput, OrganizationPage, OrganizationQuery,
};
use crate::utils::error::AuthError;

/// The organization catalog and users' memberships in it.
///
/// Catalog changes are administrative: z-auth does not authorize them, the
/// host guards them (e.g. with
/// [`Authorizer`](crate::security::authorization::Authorizer)).
///
/// A user's first membership becomes primary.  Deleting the primary
/// membership does not promote another one.
#[async_trait]
pub trait OrganizationService: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<Option<Organization>, AuthError>;

    /// One page ordered by name.  `limit` is clamped to
    /// `1..=`[`MAX_ORGANIZATION_PAGE`](crate::identity::organization::MAX_ORGANIZATION_PAGE)
    /// (`0` means
    /// [`DEFAULT_ORGANIZATION_PAGE`](crate::identity::organization::DEFAULT_ORGANIZATION_PAGE));
    /// a blank `search` is ignored.
    async fn list(&self, query: OrganizationQuery) -> Result<OrganizationPage, AuthError>;

    /// # Errors
    /// [`AuthError::InvalidProfileData`], [`AuthError::OrganizationAlreadyExists`].
    async fn create(&self, input: OrganizationInput) -> Result<Organization, AuthError>;

    /// Replace the name and description.
    async fn update(&self, id: Uuid, input: OrganizationInput) -> Result<Organization, AuthError>;

    /// Delete an organization with its departments and memberships.
    async fn delete(&self, id: Uuid) -> Result<(), AuthError>;

    /// An organization's departments ordered by name.
    async fn departments(&self, organization_id: Uuid) -> Result<Vec<Department>, AuthError>;

    async fn find_department(&self, id: Uuid) -> Result<Option<Department>, AuthError>;

    /// # Errors
    /// [`AuthError::InvalidProfileData`], [`AuthError::OrganizationNotFound`],
    /// [`AuthError::DepartmentAlreadyExists`].
    async fn create_department(
        &self,
        organization_id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError>;

    /// Replace the name and description.
    async fn update_department(
        &self,
        id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError>;

    /// Delete a department; its members stay in the organization with no
    /// department.
    async fn delete_department(&self, id: Uuid) -> Result<(), AuthError>;

    /// A user's memberships, the primary first, then oldest first.
    async fn memberships(&self, user_id: Uuid) -> Result<Vec<Membership>, AuthError>;

    /// One page of an organization's memberships, oldest first; paging as
    /// [`list`](Self::list).
    async fn members(
        &self,
        organization_id: Uuid,
        query: MemberQuery,
    ) -> Result<MembershipPage, AuthError>;

    /// Validate and add a membership.
    ///
    /// # Errors
    /// [`AuthError::OrganizationNotFound`], [`AuthError::DepartmentNotFound`]
    /// (also for a department of another organization),
    /// [`AuthError::MembershipAlreadyExists`], [`AuthError::ProfileLimitReached`],
    /// [`AuthError::InvalidProfileData`], [`AuthError::UserNotFound`].
    async fn add_membership(
        &self,
        user_id: Uuid,
        input: MembershipInput,
    ) -> Result<Membership, AuthError>;

    /// Validate and replace a membership's organization, department and
    /// title; [`AuthError::MembershipNotFound`] if the user has no such
    /// membership.
    async fn update_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
        input: MembershipInput,
    ) -> Result<Membership, AuthError>;

    /// Make a membership the user's primary one.
    async fn set_primary_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<(), AuthError>;

    async fn delete_membership(&self, user_id: Uuid, membership_id: Uuid) -> Result<(), AuthError>;
}
