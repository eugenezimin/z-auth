//! PostgreSQL implementation of z-auth's [`OrganizationRepository`].
//!
//! The statements live in [`crate::tx::organizations`].

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::pool::PoolConnection;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::organization::{
    Department, DepartmentInput, MemberQuery, Membership, MembershipPage, NewMembership,
    Organization, OrganizationInput, OrganizationPage, OrganizationQuery, OrganizationRepository,
};

use super::profile_repository::in_tx;
use crate::errors::map_sqlx_error;
use crate::tx::organizations;

/// PostgreSQL-backed organization catalog and memberships (see
/// `migrations_profile/4001_profile_schema.sql`).
pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn conn(&self) -> Result<PoolConnection<sqlx::Postgres>, AuthError> {
        self.pool.acquire().await.map_err(map_sqlx_error)
    }
}

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    async fn find_organization(&self, id: Uuid) -> Result<Option<Organization>, AuthError> {
        organizations::find_organization(&mut *self.conn().await?, id).await
    }

    async fn list_organizations(
        &self,
        query: &OrganizationQuery,
    ) -> Result<OrganizationPage, AuthError> {
        organizations::list_organizations(&mut *self.conn().await?, query).await
    }

    async fn create_organization(
        &self,
        input: OrganizationInput,
    ) -> Result<Organization, AuthError> {
        organizations::create_organization(&mut *self.conn().await?, &input).await
    }

    async fn update_organization(
        &self,
        id: Uuid,
        input: OrganizationInput,
    ) -> Result<Option<Organization>, AuthError> {
        organizations::update_organization(&mut *self.conn().await?, id, &input).await
    }

    async fn delete_organization(&self, id: Uuid) -> Result<bool, AuthError> {
        organizations::delete_organization(&mut *self.conn().await?, id).await
    }

    async fn list_departments(&self, organization_id: Uuid) -> Result<Vec<Department>, AuthError> {
        organizations::list_departments(&mut *self.conn().await?, organization_id).await
    }

    async fn find_department(&self, id: Uuid) -> Result<Option<Department>, AuthError> {
        organizations::find_department(&mut *self.conn().await?, id).await
    }

    async fn create_department(
        &self,
        organization_id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError> {
        organizations::create_department(&mut *self.conn().await?, organization_id, &input).await
    }

    async fn update_department(
        &self,
        id: Uuid,
        input: DepartmentInput,
    ) -> Result<Option<Department>, AuthError> {
        organizations::update_department(&mut *self.conn().await?, id, &input).await
    }

    async fn delete_department(&self, id: Uuid) -> Result<bool, AuthError> {
        organizations::delete_department(&mut *self.conn().await?, id).await
    }

    async fn list_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>, AuthError> {
        organizations::list_memberships(&mut *self.conn().await?, user_id).await
    }

    async fn list_members(
        &self,
        organization_id: Uuid,
        query: &MemberQuery,
    ) -> Result<MembershipPage, AuthError> {
        organizations::list_members(&mut *self.conn().await?, organization_id, query).await
    }

    async fn add_membership(
        &self,
        user_id: Uuid,
        membership: NewMembership,
    ) -> Result<Membership, AuthError> {
        in_tx!(self.pool, |tx| organizations::add_membership(
            &mut tx,
            user_id,
            &membership
        ))
    }

    async fn update_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
        membership: NewMembership,
    ) -> Result<Option<Membership>, AuthError> {
        in_tx!(self.pool, |tx| organizations::update_membership(
            &mut tx,
            user_id,
            membership_id,
            &membership
        ))
    }

    async fn set_primary_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<bool, AuthError> {
        in_tx!(self.pool, |tx| organizations::set_primary_membership(
            &mut tx,
            user_id,
            membership_id
        ))
    }

    async fn delete_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<bool, AuthError> {
        organizations::delete_membership(&mut *self.conn().await?, user_id, membership_id).await
    }
}
