//! Framework-agnostic endpoint handlers.
//!
//! Each handler takes a service trait object plus a request DTO and returns a
//! response DTO or an [`ApiError`].  The host application wraps them in its
//! own web framework and supplies the [`ClientContext`] (real client IP,
//! user agent):
//!
//! ```rust,ignore
//! // axum example (host application code)
//! async fn refresh(
//!     State(auth): State<Arc<ZAuth>>,
//!     ConnectInfo(addr): ConnectInfo<SocketAddr>,
//!     Json(req): Json<RefreshRequest>,
//! ) -> Result<Json<TokenPairResponse>, MyError> {
//!     let ctx = ClientContext::new(addr.ip());
//!     Ok(Json(handlers::refresh(auth.authentication(), req, ctx).await?))
//! }
//! ```

use uuid::Uuid;

use crate::controller::api::dto::{
    AddressRequest, AddressResponse, CompleteLoginRequest, ConfirmLinkRequest, DepartmentRequest,
    DepartmentResponse, ExternalIdentityResponse, LoginRequest, LoginResponse,
    MembershipPageResponse, MembershipRequest, MembershipResponse, OrganizationPageResponse,
    OrganizationRequest, OrganizationResponse, PermissionCatalogResponse, PhoneRequest,
    PhoneResponse, RefreshRequest, RegisterRequest, RevokeRequest, RevokeResponse,
    TokenPairResponse, UserResponse,
};
use crate::controller::api::error::ApiError;
use crate::identity::organization::{MemberQuery, OrganizationQuery, OrganizationService};
use crate::identity::profile::ProfileService;
use crate::identity::user::service::UserService;
use crate::security::authentication::linking::AccountLinkService;
use crate::security::authentication::model::ClientContext;
use crate::security::authentication::service::AuthenticationService;
use crate::security::authorization::service::PermissionService;
use crate::security::token::service::TokenRevocationService;
use crate::utils::error::AuthError;

/// `POST /register`
pub async fn register(
    users: &dyn UserService,
    req: RegisterRequest,
) -> Result<UserResponse, ApiError> {
    Ok(users.register(req.into()).await?.into())
}

/// `POST /login` — tokens, or a challenge when a second factor is needed.
pub async fn login(
    auth: &dyn AuthenticationService,
    req: LoginRequest,
    ctx: ClientContext,
) -> Result<LoginResponse, ApiError> {
    Ok(auth.login(req.into(), ctx).await?.into())
}

/// `POST /login/second-factor` — complete a login with the challenge token
/// and a code.
pub async fn complete_login(
    auth: &dyn AuthenticationService,
    req: CompleteLoginRequest,
    ctx: ClientContext,
) -> Result<TokenPairResponse, ApiError> {
    Ok(auth.complete_login(req.into(), ctx).await?.into())
}

/// `POST /login/link` — answer a `link_confirmation_required` sign-in with
/// the existing account's password: links the external identity, then
/// logs in (tokens, or a second-factor challenge).
pub async fn confirm_link(
    auth: &dyn AuthenticationService,
    req: ConfirmLinkRequest,
    ctx: ClientContext,
) -> Result<LoginResponse, ApiError> {
    Ok(auth.confirm_link(req.into(), ctx).await?.into())
}

/// `GET /me/identities` with a valid bearer access token — the external
/// identities linked to its user.
pub async fn external_identities(
    auth: &dyn AuthenticationService,
    links: &dyn AccountLinkService,
    bearer: &str,
) -> Result<Vec<ExternalIdentityResponse>, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    let identities = links.list(claims.sub).await?;
    Ok(identities.into_iter().map(Into::into).collect())
}

/// `DELETE /me/identities/{id}` with a bearer access token of a recent
/// session.  Refused for the user's last way to log in.
pub async fn unlink_external_identity(
    auth: &dyn AuthenticationService,
    links: &dyn AccountLinkService,
    bearer: &str,
    identity_id: Uuid,
) -> Result<ExternalIdentityResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(links.unlink(claims.sid, identity_id).await?.into())
}

// ── Profile (the bearer's own) ────────────────────────────────────────────────
//
// Every handler below takes a valid bearer access token and acts for its
// user.

/// `GET /me/phones` — primary first, then oldest first.
pub async fn phones(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
) -> Result<Vec<PhoneResponse>, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    let phones = profiles.phones(claims.sub).await?;
    Ok(phones.into_iter().map(Into::into).collect())
}

/// `POST /me/phones` — the first phone becomes primary.
pub async fn add_phone(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    req: PhoneRequest,
) -> Result<PhoneResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles.add_phone(claims.sub, req.into()).await?.into())
}

/// `PUT /me/phones/{id}`
pub async fn replace_phone(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    phone_id: Uuid,
    req: PhoneRequest,
) -> Result<PhoneResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles
        .replace_phone(claims.sub, phone_id, req.into())
        .await?
        .into())
}

/// `POST /me/phones/{id}/primary`
pub async fn set_primary_phone(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    phone_id: Uuid,
) -> Result<(), ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles.set_primary_phone(claims.sub, phone_id).await?)
}

/// `DELETE /me/phones/{id}`
pub async fn delete_phone(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    phone_id: Uuid,
) -> Result<(), ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles.delete_phone(claims.sub, phone_id).await?)
}

/// `GET /me/addresses` — primary first, then oldest first.
pub async fn addresses(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
) -> Result<Vec<AddressResponse>, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    let addresses = profiles.addresses(claims.sub).await?;
    Ok(addresses.into_iter().map(Into::into).collect())
}

/// `POST /me/addresses` — the first address becomes primary.
pub async fn add_address(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    req: AddressRequest,
) -> Result<AddressResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles.add_address(claims.sub, req.into()).await?.into())
}

/// `PUT /me/addresses/{id}`
pub async fn replace_address(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    address_id: Uuid,
    req: AddressRequest,
) -> Result<AddressResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles
        .replace_address(claims.sub, address_id, req.into())
        .await?
        .into())
}

/// `POST /me/addresses/{id}/primary`
pub async fn set_primary_address(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    address_id: Uuid,
) -> Result<(), ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles.set_primary_address(claims.sub, address_id).await?)
}

/// `DELETE /me/addresses/{id}`
pub async fn delete_address(
    auth: &dyn AuthenticationService,
    profiles: &dyn ProfileService,
    bearer: &str,
    address_id: Uuid,
) -> Result<(), ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(profiles.delete_address(claims.sub, address_id).await?)
}

/// `GET /me/organizations` — the bearer's memberships, primary first.
pub async fn memberships(
    auth: &dyn AuthenticationService,
    organizations: &dyn OrganizationService,
    bearer: &str,
) -> Result<Vec<MembershipResponse>, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    let memberships = organizations.memberships(claims.sub).await?;
    Ok(memberships.into_iter().map(Into::into).collect())
}

/// `POST /me/organizations` — join an organization of the catalog,
/// optionally one of its departments.  The first membership becomes
/// primary.
pub async fn add_membership(
    auth: &dyn AuthenticationService,
    organizations: &dyn OrganizationService,
    bearer: &str,
    req: MembershipRequest,
) -> Result<MembershipResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(organizations
        .add_membership(claims.sub, req.into())
        .await?
        .into())
}

/// `PUT /me/organizations/{id}`
pub async fn update_membership(
    auth: &dyn AuthenticationService,
    organizations: &dyn OrganizationService,
    bearer: &str,
    membership_id: Uuid,
    req: MembershipRequest,
) -> Result<MembershipResponse, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(organizations
        .update_membership(claims.sub, membership_id, req.into())
        .await?
        .into())
}

/// `POST /me/organizations/{id}/primary`
pub async fn set_primary_membership(
    auth: &dyn AuthenticationService,
    organizations: &dyn OrganizationService,
    bearer: &str,
    membership_id: Uuid,
) -> Result<(), ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(organizations
        .set_primary_membership(claims.sub, membership_id)
        .await?)
}

/// `DELETE /me/organizations/{id}`
pub async fn delete_membership(
    auth: &dyn AuthenticationService,
    organizations: &dyn OrganizationService,
    bearer: &str,
    membership_id: Uuid,
) -> Result<(), ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(organizations
        .delete_membership(claims.sub, membership_id)
        .await?)
}

// ── Organization catalog (administrative) ─────────────────────────────────────
//
// No bearer: the host authorizes these routes itself (e.g. with
// `Authorizer`), as it does for `revoke`.

/// `GET /organizations?search=&limit=&offset=` — ordered by name.
pub async fn organizations(
    organizations: &dyn OrganizationService,
    query: OrganizationQuery,
) -> Result<OrganizationPageResponse, ApiError> {
    Ok(organizations.list(query).await?.into())
}

/// `POST /organizations`
pub async fn create_organization(
    organizations: &dyn OrganizationService,
    req: OrganizationRequest,
) -> Result<OrganizationResponse, ApiError> {
    Ok(organizations.create(req.into()).await?.into())
}

/// `GET /organizations/{id}`
pub async fn organization(
    organizations: &dyn OrganizationService,
    organization_id: Uuid,
) -> Result<OrganizationResponse, ApiError> {
    let organization = organizations
        .find(organization_id)
        .await?
        .ok_or(AuthError::OrganizationNotFound)?;
    Ok(organization.into())
}

/// `PUT /organizations/{id}`
pub async fn update_organization(
    organizations: &dyn OrganizationService,
    organization_id: Uuid,
    req: OrganizationRequest,
) -> Result<OrganizationResponse, ApiError> {
    Ok(organizations
        .update(organization_id, req.into())
        .await?
        .into())
}

/// `DELETE /organizations/{id}` — with its departments and memberships.
pub async fn delete_organization(
    organizations: &dyn OrganizationService,
    organization_id: Uuid,
) -> Result<(), ApiError> {
    Ok(organizations.delete(organization_id).await?)
}

/// `GET /organizations/{id}/departments` — ordered by name.
pub async fn departments(
    organizations: &dyn OrganizationService,
    organization_id: Uuid,
) -> Result<Vec<DepartmentResponse>, ApiError> {
    let departments = organizations.departments(organization_id).await?;
    Ok(departments.into_iter().map(Into::into).collect())
}

/// `POST /organizations/{id}/departments`
pub async fn create_department(
    organizations: &dyn OrganizationService,
    organization_id: Uuid,
    req: DepartmentRequest,
) -> Result<DepartmentResponse, ApiError> {
    Ok(organizations
        .create_department(organization_id, req.into())
        .await?
        .into())
}

/// `PUT /departments/{id}`
pub async fn update_department(
    organizations: &dyn OrganizationService,
    department_id: Uuid,
    req: DepartmentRequest,
) -> Result<DepartmentResponse, ApiError> {
    Ok(organizations
        .update_department(department_id, req.into())
        .await?
        .into())
}

/// `DELETE /departments/{id}` — its members stay in the organization with
/// no department.
pub async fn delete_department(
    organizations: &dyn OrganizationService,
    department_id: Uuid,
) -> Result<(), ApiError> {
    Ok(organizations.delete_department(department_id).await?)
}

/// `GET /organizations/{id}/members?department_id=&limit=&offset=` —
/// oldest first.
pub async fn organization_members(
    organizations: &dyn OrganizationService,
    organization_id: Uuid,
    query: MemberQuery,
) -> Result<MembershipPageResponse, ApiError> {
    Ok(organizations.members(organization_id, query).await?.into())
}

/// `POST /refresh` — the only endpoint that touches session storage.
pub async fn refresh(
    auth: &dyn AuthenticationService,
    req: RefreshRequest,
    ctx: ClientContext,
) -> Result<TokenPairResponse, ApiError> {
    Ok(auth
        .refresh(&req.access_token, &req.refresh_token, ctx)
        .await?
        .into())
}

/// `POST /logout` with the bearer access token (may be expired).
/// Returns `true` if a session was closed.
pub async fn logout(auth: &dyn AuthenticationService, bearer: &str) -> Result<bool, ApiError> {
    Ok(auth.logout(bearer).await?)
}

/// `POST /logout/all` with a valid bearer access token.  Returns the number
/// of sessions closed.
pub async fn logout_all(auth: &dyn AuthenticationService, bearer: &str) -> Result<u64, ApiError> {
    let claims = auth.verify_access_token(bearer).await?;
    Ok(auth.logout_all(claims.sub).await?)
}

/// `POST /revocations` — accept revoked tokens / sessions / users.
pub async fn revoke(
    revocation: &dyn TokenRevocationService,
    req: RevokeRequest,
) -> Result<RevokeResponse, ApiError> {
    let targets = req.targets.into_iter().map(Into::into).collect();
    let sessions_ended = revocation.revoke_many(targets, req.reason).await?;
    Ok(RevokeResponse { sessions_ended })
}

/// `GET /permissions/catalog` — served from memory; UIs fetch it once and
/// again only when a token's `pv` is newer than their copy.
pub fn permission_catalog(
    permissions: &dyn PermissionService,
) -> Result<PermissionCatalogResponse, ApiError> {
    Ok(PermissionCatalogResponse::from(
        permissions.catalog()?.as_ref(),
    ))
}
