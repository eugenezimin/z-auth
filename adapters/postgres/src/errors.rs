//! sqlx → [`AuthError`] mapping, shared by every repository.

use z_auth::AuthError;

use crate::constants::*;

/// Map a sqlx error to a domain [`AuthError`].
///
/// Unique-constraint (`23505`) and some foreign-key (`23503`) violations
/// are translated to specific variants by constraint name; everything else
/// becomes [`AuthError::Storage`].
pub(crate) fn map_sqlx_error(e: sqlx::Error) -> AuthError {
    if let sqlx::Error::Database(ref db_err) = e
        && db_err.code().as_deref() == Some(PG_FOREIGN_KEY_VIOLATION)
    {
        match db_err.constraint().unwrap_or("") {
            CONSTRAINT_USER_PHONES_USER
            | CONSTRAINT_USER_ADDRESSES_USER
            | CONSTRAINT_MEMBERSHIPS_USER => return AuthError::UserNotFound,
            CONSTRAINT_DEPARTMENTS_ORGANIZATION | CONSTRAINT_MEMBERSHIPS_ORGANIZATION => {
                return AuthError::OrganizationNotFound;
            }
            CONSTRAINT_MEMBERSHIPS_DEPARTMENT => return AuthError::DepartmentNotFound,
            _ => {}
        }
    }
    if let sqlx::Error::Database(ref db_err) = e
        && db_err.code().as_deref() == Some(PG_UNIQUE_VIOLATION)
    {
        match db_err.constraint().unwrap_or("") {
            CONSTRAINT_IDENTITY_USERS_EMAIL => return AuthError::EmailAlreadyTaken,
            CONSTRAINT_IDENTITY_USERS_USERNAME => return AuthError::UsernameAlreadyTaken,
            CONSTRAINT_IDENTITY_ROLES_NAME | CONSTRAINT_IDENTITY_ROLES_CODE => {
                return AuthError::RoleAlreadyExists;
            }
            CONSTRAINT_SECURITY_PERMISSIONS_CODE => return AuthError::PermissionAlreadyExists,
            CONSTRAINT_USER_ROLE_ACTIVE => return AuthError::RoleAlreadyAssigned,
            CONSTRAINT_MFA_FACTORS_ACTIVE => return AuthError::SecondFactorAlreadyEnrolled,
            CONSTRAINT_EXTERNAL_IDENTITIES_SUBJECT => {
                return AuthError::ExternalIdentityLinkedElsewhere;
            }
            CONSTRAINT_USER_PHONES_NUMBER => return AuthError::PhoneAlreadyAdded,
            CONSTRAINT_ORGANIZATIONS_NAME => return AuthError::OrganizationAlreadyExists,
            CONSTRAINT_DEPARTMENTS_NAME => return AuthError::DepartmentAlreadyExists,
            CONSTRAINT_MEMBERSHIPS_MEMBER => return AuthError::MembershipAlreadyExists,
            _ => {}
        }
    }
    AuthError::Storage(e.to_string())
}
