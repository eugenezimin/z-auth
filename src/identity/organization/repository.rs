//! Organization repository port.

use async_trait::async_trait;
use uuid::Uuid;

use crate::identity::organization::model::{
    Department, DepartmentInput, MemberQuery, Membership, MembershipPage, NewMembership,
    Organization, OrganizationInput, OrganizationPage, OrganizationQuery,
};
use crate::utils::error::AuthError;

/// Persistence contract for the organization catalog and users'
/// memberships.  Inputs arrive validated and trimmed.
///
/// - Organization names are unique case-insensitively
///   ([`AuthError::OrganizationAlreadyExists`]); department names are unique
///   within their organization the same way
///   ([`AuthError::DepartmentAlreadyExists`]).
/// - A membership's department belongs to the membership's organization
///   ([`AuthError::DepartmentNotFound`] otherwise), and a user holds a given
///   `(organization, department)` pair once
///   ([`AuthError::MembershipAlreadyExists`]); no department is a value of
///   its own here.
/// - Every call on one membership is scoped by `(user_id, id)`.  Writing one
///   with `is_primary = true` clears the user's previous primary membership
///   in the same atomic step; `false` on an update leaves the flag
///   unchanged.
/// - Deleting an organization deletes its departments and memberships.
///   Deleting a department keeps its memberships, with no department.
///   Deleting a user deletes their memberships.
///
/// Map driver errors to [`AuthError::Storage`], an unknown user to
/// [`AuthError::UserNotFound`] and an unknown organization to
/// [`AuthError::OrganizationNotFound`].
#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn find_organization(&self, id: Uuid) -> Result<Option<Organization>, AuthError>;

    /// One page ordered by name, case-insensitively (ties by id).
    async fn list_organizations(
        &self,
        query: &OrganizationQuery,
    ) -> Result<OrganizationPage, AuthError>;

    async fn create_organization(
        &self,
        input: OrganizationInput,
    ) -> Result<Organization, AuthError>;

    /// `Ok(None)` if no such organization exists.
    async fn update_organization(
        &self,
        id: Uuid,
        input: OrganizationInput,
    ) -> Result<Option<Organization>, AuthError>;

    /// Returns `false` if no such organization exists.
    async fn delete_organization(&self, id: Uuid) -> Result<bool, AuthError>;

    /// An organization's departments ordered by name, case-insensitively.
    async fn list_departments(&self, organization_id: Uuid) -> Result<Vec<Department>, AuthError>;

    async fn find_department(&self, id: Uuid) -> Result<Option<Department>, AuthError>;

    async fn create_department(
        &self,
        organization_id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError>;

    /// `Ok(None)` if no such department exists.
    async fn update_department(
        &self,
        id: Uuid,
        input: DepartmentInput,
    ) -> Result<Option<Department>, AuthError>;

    /// Returns `false` if no such department exists.
    async fn delete_department(&self, id: Uuid) -> Result<bool, AuthError>;

    /// A user's memberships, the primary first, then oldest first.
    async fn list_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>, AuthError>;

    /// One page of an organization's memberships, oldest first (ties by id).
    async fn list_members(
        &self,
        organization_id: Uuid,
        query: &MemberQuery,
    ) -> Result<MembershipPage, AuthError>;

    async fn add_membership(
        &self,
        user_id: Uuid,
        membership: NewMembership,
    ) -> Result<Membership, AuthError>;

    /// Replace a membership's organization, department and title.
    /// `Ok(None)` if the user has no such membership.
    async fn update_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
        membership: NewMembership,
    ) -> Result<Option<Membership>, AuthError>;

    /// Returns `false` if the user has no such membership.
    async fn set_primary_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<bool, AuthError>;

    /// Returns `false` if the user has no such membership.
    async fn delete_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<bool, AuthError>;
}
