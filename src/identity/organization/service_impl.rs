//! Organization service implementation.
//!
//! [`OrganizationServiceImpl`] validates inputs, clamps paging, checks that
//! a membership's department belongs to its organization, enforces the
//! per-user membership limit and makes a user's first membership primary,
//! then hands the result to the [`OrganizationRepository`] port.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::conf::constants::MAX_MEMBERSHIPS_PER_USER;
use crate::identity::organization::model::{
    DEFAULT_ORGANIZATION_PAGE, Department, DepartmentInput, MAX_ORGANIZATION_PAGE, MemberQuery,
    Membership, MembershipInput, MembershipPage, NewMembership, Organization, OrganizationInput,
    OrganizationPage, OrganizationQuery,
};
use crate::identity::organization::repository::OrganizationRepository;
use crate::identity::organization::service::OrganizationService;
use crate::identity::organization::validation::{
    validate_department, validate_membership, validate_organization,
};
use crate::utils::error::AuthError;

/// Default implementation of [`OrganizationService`].
pub struct OrganizationServiceImpl {
    repo: Option<Arc<dyn OrganizationRepository>>,
}

impl OrganizationServiceImpl {
    /// Without `repo` every call fails with [`AuthError::Config`].
    pub fn new(repo: Option<Arc<dyn OrganizationRepository>>) -> Self {
        Self { repo }
    }

    fn repo(&self) -> Result<&dyn OrganizationRepository, AuthError> {
        self.repo.as_deref().ok_or_else(|| {
            AuthError::Config("missing adapter: organizations (ZAuthBuilder::organizations)".into())
        })
    }

    async fn require_organization(&self, id: Uuid) -> Result<(), AuthError> {
        match self.repo()?.find_organization(id).await? {
            Some(_) => Ok(()),
            None => Err(AuthError::OrganizationNotFound),
        }
    }

    /// The organization exists and the department, if any, is one of its.
    async fn check_placement(&self, membership: &NewMembership) -> Result<(), AuthError> {
        match membership.department_id {
            Some(department_id) => {
                let department = self.repo()?.find_department(department_id).await?;
                match department {
                    Some(d) if d.organization_id == membership.organization_id => Ok(()),
                    Some(_) | None => {
                        self.require_organization(membership.organization_id)
                            .await?;
                        Err(AuthError::DepartmentNotFound)
                    }
                }
            }
            None => self.require_organization(membership.organization_id).await,
        }
    }
}

fn clamp_page(limit: u32) -> u32 {
    match limit {
        0 => DEFAULT_ORGANIZATION_PAGE,
        n => n.min(MAX_ORGANIZATION_PAGE),
    }
}

#[async_trait]
impl OrganizationService for OrganizationServiceImpl {
    async fn find(&self, id: Uuid) -> Result<Option<Organization>, AuthError> {
        self.repo()?.find_organization(id).await
    }

    async fn list(&self, mut query: OrganizationQuery) -> Result<OrganizationPage, AuthError> {
        query.limit = clamp_page(query.limit);
        query.search = query
            .search
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        self.repo()?.list_organizations(&query).await
    }

    async fn create(&self, input: OrganizationInput) -> Result<Organization, AuthError> {
        let input = validate_organization(input)?;
        self.repo()?.create_organization(input).await
    }

    async fn update(&self, id: Uuid, input: OrganizationInput) -> Result<Organization, AuthError> {
        let input = validate_organization(input)?;
        self.repo()?
            .update_organization(id, input)
            .await?
            .ok_or(AuthError::OrganizationNotFound)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AuthError> {
        match self.repo()?.delete_organization(id).await? {
            true => Ok(()),
            false => Err(AuthError::OrganizationNotFound),
        }
    }

    async fn departments(&self, organization_id: Uuid) -> Result<Vec<Department>, AuthError> {
        self.require_organization(organization_id).await?;
        self.repo()?.list_departments(organization_id).await
    }

    async fn find_department(&self, id: Uuid) -> Result<Option<Department>, AuthError> {
        self.repo()?.find_department(id).await
    }

    async fn create_department(
        &self,
        organization_id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError> {
        let input = validate_department(input)?;
        self.repo()?.create_department(organization_id, input).await
    }

    async fn update_department(
        &self,
        id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError> {
        let input = validate_department(input)?;
        self.repo()?
            .update_department(id, input)
            .await?
            .ok_or(AuthError::DepartmentNotFound)
    }

    async fn delete_department(&self, id: Uuid) -> Result<(), AuthError> {
        match self.repo()?.delete_department(id).await? {
            true => Ok(()),
            false => Err(AuthError::DepartmentNotFound),
        }
    }

    async fn memberships(&self, user_id: Uuid) -> Result<Vec<Membership>, AuthError> {
        self.repo()?.list_memberships(user_id).await
    }

    async fn members(
        &self,
        organization_id: Uuid,
        mut query: MemberQuery,
    ) -> Result<MembershipPage, AuthError> {
        self.require_organization(organization_id).await?;
        query.limit = clamp_page(query.limit);
        self.repo()?.list_members(organization_id, &query).await
    }

    async fn add_membership(
        &self,
        user_id: Uuid,
        input: MembershipInput,
    ) -> Result<Membership, AuthError> {
        let repo = self.repo()?;
        let mut membership = validate_membership(input)?;
        self.check_placement(&membership).await?;
        let existing = repo.list_memberships(user_id).await?;
        if existing.len() >= MAX_MEMBERSHIPS_PER_USER {
            return Err(AuthError::ProfileLimitReached(format!(
                "a user may hold at most {MAX_MEMBERSHIPS_PER_USER} organization memberships"
            )));
        }
        membership.is_primary |= existing.is_empty();
        repo.add_membership(user_id, membership).await
    }

    async fn update_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
        input: MembershipInput,
    ) -> Result<Membership, AuthError> {
        let membership = validate_membership(input)?;
        self.check_placement(&membership).await?;
        self.repo()?
            .update_membership(user_id, membership_id, membership)
            .await?
            .ok_or(AuthError::MembershipNotFound)
    }

    async fn set_primary_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<(), AuthError> {
        match self
            .repo()?
            .set_primary_membership(user_id, membership_id)
            .await?
        {
            true => Ok(()),
            false => Err(AuthError::MembershipNotFound),
        }
    }

    async fn delete_membership(&self, user_id: Uuid, membership_id: Uuid) -> Result<(), AuthError> {
        match self
            .repo()?
            .delete_membership(user_id, membership_id)
            .await?
        {
            true => Ok(()),
            false => Err(AuthError::MembershipNotFound),
        }
    }
}
