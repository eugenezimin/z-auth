//! Conversions between API DTOs and domain models.

use crate::controller::api::dto::{
    AddressRequest, AddressResponse, CatalogRefDto, ChallengeDeliveryDto, CompleteLoginRequest,
    ConfirmLinkRequest, DepartmentRequest, DepartmentResponse, ExternalIdentityResponse,
    ExternalLoginResponse, LinkChallengeResponse, LoginChallengeResponse, LoginRequest,
    LoginResponse, MembershipPageResponse, MembershipRequest, MembershipResponse,
    OrganizationPageResponse, OrganizationRequest, OrganizationResponse, PermissionCatalogResponse,
    PermissionDto, PermissionOptionDto, PhoneRequest, PhoneResponse, RegisterRequest,
    RevokeTargetDto, TokenPairResponse, UserResponse,
};
use crate::identity::external::ExternalIdentity;
use crate::identity::organization::{
    Department, DepartmentInput, DepartmentRef, Membership, MembershipInput, MembershipPage,
    Organization, OrganizationInput, OrganizationPage, OrganizationRef,
};
use crate::identity::profile::{Address, AddressInput, Phone, PhoneInput};
use crate::identity::user::model::{RegisterUser, User};
use crate::security::authentication::model::{
    ChallengeDelivery, Credentials, ExternalLoginOutcome, LinkChallenge, LinkConfirmation,
    LoginChallenge, LoginOutcome, SecondFactorProof, TokenPair,
};
use crate::security::authorization::model::{Permission, PermissionCatalog, PermissionKind};
use crate::security::token::model::RevokeTarget;

impl From<RegisterRequest> for RegisterUser {
    fn from(r: RegisterRequest) -> Self {
        Self {
            email: r.email,
            password: r.password,
            username: r.username,
            first_name: r.first_name,
            last_name: r.last_name,
        }
    }
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            username: u.username,
            first_name: u.first_name,
            last_name: u.last_name,
            avatar_url: u.avatar_url,
            is_active: u.is_active,
            is_verified: u.is_verified,
            created_at: u.created_at,
        }
    }
}

impl From<LoginRequest> for Credentials {
    fn from(r: LoginRequest) -> Self {
        Self {
            email: r.email,
            password: r.password,
        }
    }
}

impl From<TokenPair> for TokenPairResponse {
    fn from(p: TokenPair) -> Self {
        Self {
            session_id: p.session_id,
            access_token: p.access_token,
            access_expires_at: p.access_expires_at,
            refresh_token: p.refresh_token,
            refresh_expires_at: p.refresh_expires_at,
        }
    }
}

impl From<LoginOutcome> for LoginResponse {
    fn from(o: LoginOutcome) -> Self {
        match o {
            LoginOutcome::Tokens(pair) => Self::Authenticated(pair.into()),
            LoginOutcome::SecondFactor(challenge) => Self::SecondFactorRequired(challenge.into()),
        }
    }
}

impl From<ExternalLoginOutcome> for ExternalLoginResponse {
    fn from(o: ExternalLoginOutcome) -> Self {
        match o {
            ExternalLoginOutcome::Tokens(pair) => Self::Authenticated(pair.into()),
            ExternalLoginOutcome::SecondFactor(challenge) => {
                Self::SecondFactorRequired(challenge.into())
            }
            ExternalLoginOutcome::LinkConfirmation(challenge) => {
                Self::LinkConfirmationRequired(challenge.into())
            }
        }
    }
}

impl From<LinkChallenge> for LinkChallengeResponse {
    fn from(c: LinkChallenge) -> Self {
        Self {
            challenge_token: c.challenge_token,
            expires_at: c.expires_at,
            email: c.email,
        }
    }
}

impl From<ConfirmLinkRequest> for LinkConfirmation {
    fn from(r: ConfirmLinkRequest) -> Self {
        Self {
            challenge_token: r.challenge_token,
            password: r.password,
        }
    }
}

impl From<ExternalIdentity> for ExternalIdentityResponse {
    fn from(i: ExternalIdentity) -> Self {
        Self {
            id: i.id,
            provider: i.provider,
            email: i.email,
            linked_at: i.created_at,
            last_login_at: i.last_login_at,
        }
    }
}

impl From<PhoneRequest> for PhoneInput {
    fn from(r: PhoneRequest) -> Self {
        Self {
            label: r.label,
            number: r.number,
            is_primary: r.is_primary,
        }
    }
}

impl From<Phone> for PhoneResponse {
    fn from(p: Phone) -> Self {
        Self {
            id: p.id,
            label: p.label,
            number: p.number,
            is_primary: p.is_primary,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

impl From<AddressRequest> for AddressInput {
    fn from(r: AddressRequest) -> Self {
        Self {
            label: r.label,
            line1: r.line1,
            line2: r.line2,
            locality: r.locality,
            region: r.region,
            postal_code: r.postal_code,
            country_code: r.country_code,
            is_primary: r.is_primary,
        }
    }
}

impl From<Address> for AddressResponse {
    fn from(a: Address) -> Self {
        Self {
            id: a.id,
            label: a.label,
            line1: a.line1,
            line2: a.line2,
            locality: a.locality,
            region: a.region,
            postal_code: a.postal_code,
            country_code: a.country_code,
            is_primary: a.is_primary,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

impl From<OrganizationRequest> for OrganizationInput {
    fn from(r: OrganizationRequest) -> Self {
        Self {
            name: r.name,
            description: r.description,
        }
    }
}

impl From<Organization> for OrganizationResponse {
    fn from(o: Organization) -> Self {
        Self {
            id: o.id,
            name: o.name,
            description: o.description,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}

impl From<OrganizationPage> for OrganizationPageResponse {
    fn from(p: OrganizationPage) -> Self {
        Self {
            items: p.items.into_iter().map(Into::into).collect(),
            total: p.total,
        }
    }
}

impl From<DepartmentRequest> for DepartmentInput {
    fn from(r: DepartmentRequest) -> Self {
        Self {
            name: r.name,
            description: r.description,
        }
    }
}

impl From<Department> for DepartmentResponse {
    fn from(d: Department) -> Self {
        Self {
            id: d.id,
            organization_id: d.organization_id,
            name: d.name,
            description: d.description,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

impl From<MembershipRequest> for MembershipInput {
    fn from(r: MembershipRequest) -> Self {
        Self {
            organization_id: r.organization_id,
            department_id: r.department_id,
            title: r.title,
            is_primary: r.is_primary,
        }
    }
}

impl From<OrganizationRef> for CatalogRefDto {
    fn from(r: OrganizationRef) -> Self {
        Self {
            id: r.id,
            name: r.name,
        }
    }
}

impl From<DepartmentRef> for CatalogRefDto {
    fn from(r: DepartmentRef) -> Self {
        Self {
            id: r.id,
            name: r.name,
        }
    }
}

impl From<Membership> for MembershipResponse {
    fn from(m: Membership) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            organization: m.organization.into(),
            department: m.department.map(Into::into),
            title: m.title,
            is_primary: m.is_primary,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

impl From<MembershipPage> for MembershipPageResponse {
    fn from(p: MembershipPage) -> Self {
        Self {
            items: p.items.into_iter().map(Into::into).collect(),
            total: p.total,
        }
    }
}

impl From<LoginChallenge> for LoginChallengeResponse {
    fn from(c: LoginChallenge) -> Self {
        Self {
            challenge_token: c.challenge_token,
            expires_at: c.expires_at,
            methods: c.methods,
            delivery: c.delivery.map(Into::into),
        }
    }
}

impl From<ChallengeDelivery> for ChallengeDeliveryDto {
    fn from(d: ChallengeDelivery) -> Self {
        Self {
            method: d.method,
            destination_hint: d.destination_hint,
            expires_at: d.expires_at,
        }
    }
}

impl From<CompleteLoginRequest> for SecondFactorProof {
    fn from(r: CompleteLoginRequest) -> Self {
        Self {
            challenge_token: r.challenge_token,
            method: r.method,
            code: r.code,
        }
    }
}

impl From<RevokeTargetDto> for RevokeTarget {
    fn from(t: RevokeTargetDto) -> Self {
        match t {
            RevokeTargetDto::AccessToken(token) => Self::AccessToken(token),
            RevokeTargetDto::RefreshToken(token) => Self::RefreshToken(token),
            RevokeTargetDto::Session(id) => Self::Session(id),
            RevokeTargetDto::User(id) => Self::User(id),
        }
    }
}

impl From<&PermissionCatalog> for PermissionCatalogResponse {
    fn from(c: &PermissionCatalog) -> Self {
        Self {
            version: c.version,
            permissions: c.permissions.iter().map(Into::into).collect(),
        }
    }
}

impl From<&Permission> for PermissionDto {
    fn from(p: &Permission) -> Self {
        let (kind, max_length) = match p.kind {
            PermissionKind::Bool => ("bool", None),
            PermissionKind::Single => ("single", None),
            PermissionKind::Multi => ("multi", None),
            PermissionKind::Text { max_length } => ("text", Some(max_length)),
        };
        Self {
            code: p.code.clone(),
            kind: kind.into(),
            description: p.description.clone(),
            position: p.position,
            max_length,
            options: p
                .options
                .iter()
                .map(|o| PermissionOptionDto {
                    code: o.code.clone(),
                    position: o.position,
                })
                .collect(),
        }
    }
}
