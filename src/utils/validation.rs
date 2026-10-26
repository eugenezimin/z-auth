//! Validation shared across contexts: the codes of roles and permissions,
//! and the free-text fields of profiles and organizations.

use crate::conf::constants::MAX_CODE_LEN;
use crate::utils::error::AuthError;

/// Role, permission and option codes: `^[a-z][a-z0-9_.:-]*$`, at most
/// [`MAX_CODE_LEN`] characters.  Codes are immutable identifiers that end
/// up in tokens and application code.
pub fn validate_code(code: &str) -> Result<(), AuthError> {
    let mut chars = code.chars();
    let valid_start = chars.next().is_some_and(|c| c.is_ascii_lowercase());
    let valid_rest = chars.all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | ':' | '-')
    });
    if valid_start && valid_rest && code.len() <= MAX_CODE_LEN {
        Ok(())
    } else {
        Err(AuthError::InvalidCode(format!(
            "'{code}' must match ^[a-z][a-z0-9_.:-]*$ and be at most {MAX_CODE_LEN} characters"
        )))
    }
}

/// A required free-text field: trimmed, 1 to `max` characters, no control
/// characters.  `field` names it in the error.
pub fn required_text(field: &str, value: &str, max: usize) -> Result<String, AuthError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AuthError::InvalidProfileData(format!(
            "{field} is required"
        )));
    }
    check_text(field, value, max)?;
    Ok(value.to_owned())
}

/// An optional free-text field: trimmed, a blank value is `None`, otherwise
/// as [`required_text`].
pub fn optional_text(
    field: &str,
    value: Option<&str>,
    max: usize,
) -> Result<Option<String>, AuthError> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(value) => {
            check_text(field, value, max)?;
            Ok(Some(value.to_owned()))
        }
        None => Ok(None),
    }
}

fn check_text(field: &str, value: &str, max: usize) -> Result<(), AuthError> {
    if value.chars().count() > max {
        return Err(AuthError::InvalidProfileData(format!(
            "{field} must be at most {max} characters"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(AuthError::InvalidProfileData(format!(
            "{field} must not contain control characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts() {
        assert_eq!(required_text("name", "  Acme  ", 10).unwrap(), "Acme");
        assert_eq!(required_text("name", "Ünïcødé", 7).unwrap(), "Ünïcødé");
        for bad in ["", "   ", "12345678901", "tab\there"] {
            assert!(required_text("name", bad, 10).is_err(), "{bad:?}");
        }
        assert_eq!(optional_text("line2", None, 10).unwrap(), None);
        assert_eq!(optional_text("line2", Some("  "), 10).unwrap(), None);
        assert_eq!(
            optional_text("line2", Some(" Apt 4 "), 10)
                .unwrap()
                .as_deref(),
            Some("Apt 4")
        );
        assert!(optional_text("line2", Some("12345678901"), 10).is_err());
    }

    #[test]
    fn codes() {
        for ok in ["admin", "reports.export", "a1_b-c:d"] {
            assert!(validate_code(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "Admin",
            "1abc",
            "has space",
            "ünï",
            &"a".repeat(MAX_CODE_LEN + 1),
        ] {
            assert!(validate_code(bad).is_err(), "{bad}");
        }
    }
}
