//! PostgreSQL implementation of z-auth's [`OrganizationRepository`].  All
//! SQL lives in [`organization_queries`](super::organization_queries).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use z_auth::AuthError;
use z_auth::identity::organization::{
    Department, DepartmentInput, DepartmentRef, MemberQuery, Membership, MembershipPage,
    NewMembership, Organization, OrganizationInput, OrganizationPage, OrganizationQuery,
    OrganizationRef, OrganizationRepository,
};

use super::organization_queries as q;
use super::{begin_for_user, delete_item, set_primary};
use crate::errors::map_sqlx_error;

/// PostgreSQL-backed organization catalog and memberships (see
/// `migrations_profile/4001_profile_schema.sql`).
pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct OrganizationRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrganizationRow> for Organization {
    fn from(r: OrganizationRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DepartmentRow {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DepartmentRow> for Department {
    fn from(r: DepartmentRow) -> Self {
        Self {
            id: r.id,
            organization_id: r.organization_id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// A membership joined with its organization's and department's names.
#[derive(sqlx::FromRow)]
struct MembershipRow {
    id: Uuid,
    user_id: Uuid,
    organization_id: Uuid,
    organization_name: String,
    department_id: Option<Uuid>,
    department_name: Option<String>,
    title: Option<String>,
    is_primary: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<MembershipRow> for Membership {
    fn from(r: MembershipRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            organization: OrganizationRef {
                id: r.organization_id,
                name: r.organization_name,
            },
            department: r
                .department_id
                .zip(r.department_name)
                .map(|(id, name)| DepartmentRef { id, name }),
            title: r.title,
            is_primary: r.is_primary,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    async fn find_organization(&self, id: Uuid) -> Result<Option<Organization>, AuthError> {
        let row: Option<OrganizationRow> = sqlx::query_as(q::FIND_ORGANIZATION)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.map(Into::into))
    }

    async fn list_organizations(
        &self,
        query: &OrganizationQuery,
    ) -> Result<OrganizationPage, AuthError> {
        let rows: Vec<OrganizationRow> = sqlx::query_as(q::LIST_ORGANIZATIONS)
            .bind(&query.search)
            .bind(i64::from(query.limit))
            .bind(i64::from(query.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        let (total,): (i64,) = sqlx::query_as(q::COUNT_ORGANIZATIONS)
            .bind(&query.search)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(OrganizationPage {
            items: rows.into_iter().map(Into::into).collect(),
            total: total as u64,
        })
    }

    async fn create_organization(
        &self,
        input: OrganizationInput,
    ) -> Result<Organization, AuthError> {
        let row: OrganizationRow = sqlx::query_as(q::INSERT_ORGANIZATION)
            .bind(&input.name)
            .bind(&input.description)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.into())
    }

    async fn update_organization(
        &self,
        id: Uuid,
        input: OrganizationInput,
    ) -> Result<Option<Organization>, AuthError> {
        let row: Option<OrganizationRow> = sqlx::query_as(q::UPDATE_ORGANIZATION)
            .bind(id)
            .bind(&input.name)
            .bind(&input.description)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.map(Into::into))
    }

    async fn delete_organization(&self, id: Uuid) -> Result<bool, AuthError> {
        let result = sqlx::query(q::DELETE_ORGANIZATION)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_departments(&self, organization_id: Uuid) -> Result<Vec<Department>, AuthError> {
        let rows: Vec<DepartmentRow> = sqlx::query_as(q::LIST_DEPARTMENTS)
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_department(&self, id: Uuid) -> Result<Option<Department>, AuthError> {
        let row: Option<DepartmentRow> = sqlx::query_as(q::FIND_DEPARTMENT)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.map(Into::into))
    }

    async fn create_department(
        &self,
        organization_id: Uuid,
        input: DepartmentInput,
    ) -> Result<Department, AuthError> {
        let row: DepartmentRow = sqlx::query_as(q::INSERT_DEPARTMENT)
            .bind(organization_id)
            .bind(&input.name)
            .bind(&input.description)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.into())
    }

    async fn update_department(
        &self,
        id: Uuid,
        input: DepartmentInput,
    ) -> Result<Option<Department>, AuthError> {
        let row: Option<DepartmentRow> = sqlx::query_as(q::UPDATE_DEPARTMENT)
            .bind(id)
            .bind(&input.name)
            .bind(&input.description)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.map(Into::into))
    }

    async fn delete_department(&self, id: Uuid) -> Result<bool, AuthError> {
        let result = sqlx::query(q::DELETE_DEPARTMENT)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>, AuthError> {
        let rows: Vec<MembershipRow> = sqlx::query_as(q::LIST_MEMBERSHIPS)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_members(
        &self,
        organization_id: Uuid,
        query: &MemberQuery,
    ) -> Result<MembershipPage, AuthError> {
        let rows: Vec<MembershipRow> = sqlx::query_as(q::LIST_MEMBERS)
            .bind(organization_id)
            .bind(query.department_id)
            .bind(i64::from(query.limit))
            .bind(i64::from(query.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        let (total,): (i64,) = sqlx::query_as(q::COUNT_MEMBERS)
            .bind(organization_id)
            .bind(query.department_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(MembershipPage {
            items: rows.into_iter().map(Into::into).collect(),
            total: total as u64,
        })
    }

    async fn add_membership(
        &self,
        user_id: Uuid,
        membership: NewMembership,
    ) -> Result<Membership, AuthError> {
        let clear = membership.is_primary.then_some(q::CLEAR_PRIMARY_MEMBERSHIP);
        let mut tx = begin_for_user(&self.pool, user_id, clear)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        let row: MembershipRow = sqlx::query_as(q::INSERT_MEMBERSHIP)
            .bind(user_id)
            .bind(membership.organization_id)
            .bind(membership.department_id)
            .bind(&membership.title)
            .bind(membership.is_primary)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(row.into())
    }

    async fn update_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
        membership: NewMembership,
    ) -> Result<Option<Membership>, AuthError> {
        let clear = membership.is_primary.then_some(q::CLEAR_PRIMARY_MEMBERSHIP);
        let Some(mut tx) = begin_for_user(&self.pool, user_id, clear).await? else {
            return Ok(None);
        };
        let row: Option<MembershipRow> = sqlx::query_as(q::UPDATE_MEMBERSHIP)
            .bind(membership_id)
            .bind(user_id)
            .bind(membership.organization_id)
            .bind(membership.department_id)
            .bind(&membership.title)
            .bind(membership.is_primary)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        // No such membership: dropping `tx` rolls the clearing back.
        if row.is_some() {
            tx.commit().await.map_err(map_sqlx_error)?;
        }
        Ok(row.map(Into::into))
    }

    async fn set_primary_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<bool, AuthError> {
        set_primary(
            &self.pool,
            user_id,
            membership_id,
            q::CLEAR_PRIMARY_MEMBERSHIP,
            q::SET_PRIMARY_MEMBERSHIP,
        )
        .await
    }

    async fn delete_membership(
        &self,
        user_id: Uuid,
        membership_id: Uuid,
    ) -> Result<bool, AuthError> {
        delete_item(&self.pool, user_id, membership_id, q::DELETE_MEMBERSHIP).await
    }
}
