//! Organization input validation.

use crate::conf::constants::{MAX_PROFILE_DESCRIPTION_LEN, MAX_PROFILE_TEXT_LEN};
use crate::identity::organization::model::{
    DepartmentInput, MembershipInput, NewMembership, OrganizationInput,
};
use crate::utils::error::AuthError;
use crate::utils::validation::{optional_text, required_text};

/// Trim an organization's name (required) and description.
pub fn validate_organization(input: OrganizationInput) -> Result<OrganizationInput, AuthError> {
    Ok(OrganizationInput {
        name: required_text("name", &input.name, MAX_PROFILE_TEXT_LEN)?,
        description: optional_text(
            "description",
            input.description.as_deref(),
            MAX_PROFILE_DESCRIPTION_LEN,
        )?,
    })
}

/// Trim a department's name (required) and description.
pub fn validate_department(input: DepartmentInput) -> Result<DepartmentInput, AuthError> {
    Ok(DepartmentInput {
        name: required_text("name", &input.name, MAX_PROFILE_TEXT_LEN)?,
        description: optional_text(
            "description",
            input.description.as_deref(),
            MAX_PROFILE_DESCRIPTION_LEN,
        )?,
    })
}

/// Trim a membership's job title; a blank one is dropped.
pub fn validate_membership(input: MembershipInput) -> Result<NewMembership, AuthError> {
    Ok(NewMembership {
        organization_id: input.organization_id,
        department_id: input.department_id,
        title: optional_text("title", input.title.as_deref(), MAX_PROFILE_TEXT_LEN)?,
        is_primary: input.is_primary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organizations() {
        let org = validate_organization(OrganizationInput {
            name: "  Acme Corp ".into(),
            description: Some("   ".into()),
        })
        .unwrap();
        assert_eq!(org.name, "Acme Corp");
        assert_eq!(org.description, None);
        assert!(validate_organization(OrganizationInput::default()).is_err());
        assert!(
            validate_department(DepartmentInput {
                name: "x".repeat(MAX_PROFILE_TEXT_LEN + 1),
                description: None,
            })
            .is_err()
        );
    }

    #[test]
    fn memberships() {
        let m = validate_membership(MembershipInput {
            title: Some(" Engineer ".into()),
            ..MembershipInput::default()
        })
        .unwrap();
        assert_eq!(m.title.as_deref(), Some("Engineer"));
    }
}
