//! Profile input validation.

use crate::conf::constants::{MAX_POSTAL_CODE_LEN, MAX_PROFILE_LABEL_LEN, MAX_PROFILE_TEXT_LEN};
use crate::identity::profile::model::{AddressInput, NewAddress, NewPhone, PhoneInput};
use crate::utils::error::AuthError;
use crate::utils::validation::{optional_text, required_text};

/// Canonical form of a phone number: E.164, `+` followed by 7 to 15 digits,
/// the first not `0`.  Spaces, `-`, `.`, `(` and `)` are dropped; the
/// number must already carry its country code.
pub fn normalize_phone(number: &str) -> Result<String, AuthError> {
    let compact: String = number
        .trim()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '.' | '(' | ')'))
        .collect();
    let valid = match compact.strip_prefix('+') {
        Some(digits) => {
            (7..=15).contains(&digits.len())
                && digits.bytes().all(|b| b.is_ascii_digit())
                && !digits.starts_with('0')
        }
        None => false,
    };
    if valid {
        Ok(compact)
    } else {
        Err(AuthError::InvalidPhoneNumber(format!(
            "'{number}' must be in international format: + and 7 to 15 digits"
        )))
    }
}

/// A phone or address label: trimmed, 1 to
/// [`MAX_PROFILE_LABEL_LEN`] characters, display case kept.
pub fn normalize_label(label: &str) -> Result<String, AuthError> {
    required_text("label", label, MAX_PROFILE_LABEL_LEN)
}

/// An ISO 3166-1 alpha-2 country code, uppercased (`de` → `DE`).  The
/// format is checked, not membership in the ISO list.
pub fn normalize_country_code(code: &str) -> Result<String, AuthError> {
    let code = code.trim().to_ascii_uppercase();
    if code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase()) {
        Ok(code)
    } else {
        Err(AuthError::InvalidProfileData(format!(
            "country code '{code}' must be two ISO 3166-1 letters"
        )))
    }
}

/// Validate and normalize a [`PhoneInput`].
pub fn validate_phone(input: PhoneInput) -> Result<NewPhone, AuthError> {
    Ok(NewPhone {
        label: normalize_label(&input.label)?,
        number: normalize_phone(&input.number)?,
        is_primary: input.is_primary,
    })
}

/// Validate and normalize an [`AddressInput`].
pub fn validate_address(input: AddressInput) -> Result<NewAddress, AuthError> {
    Ok(NewAddress {
        label: normalize_label(&input.label)?,
        line1: required_text("line1", &input.line1, MAX_PROFILE_TEXT_LEN)?,
        line2: optional_text("line2", input.line2.as_deref(), MAX_PROFILE_TEXT_LEN)?,
        locality: optional_text("locality", input.locality.as_deref(), MAX_PROFILE_TEXT_LEN)?,
        region: optional_text("region", input.region.as_deref(), MAX_PROFILE_TEXT_LEN)?,
        postal_code: optional_text(
            "postal_code",
            input.postal_code.as_deref(),
            MAX_POSTAL_CODE_LEN,
        )?,
        country_code: normalize_country_code(&input.country_code)?,
        is_primary: input.is_primary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones() {
        for (raw, normalized) in [
            ("+1 (555) 123-4567", "+15551234567"),
            ("  +49.30.1234567 ", "+49301234567"),
            ("+4412345", "+4412345"),
        ] {
            assert_eq!(normalize_phone(raw).unwrap(), normalized, "{raw}");
        }
        for bad in [
            "",
            "5551234567",
            "+0123456789",
            "+123456",
            "+1234567890123456",
            "+1555abc4567",
            "++15551234567",
        ] {
            assert!(normalize_phone(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn labels() {
        assert_eq!(normalize_label("  Work ").unwrap(), "Work");
        assert_eq!(normalize_label("summer house").unwrap(), "summer house");
        assert!(normalize_label(" ").is_err());
        assert!(normalize_label(&"a".repeat(MAX_PROFILE_LABEL_LEN + 1)).is_err());
    }

    #[test]
    fn country_codes() {
        assert_eq!(normalize_country_code(" de ").unwrap(), "DE");
        for bad in ["", "D", "DEU", "D1", "ü1"] {
            assert!(normalize_country_code(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn addresses() {
        let address = validate_address(AddressInput {
            label: "home".into(),
            line1: " 1 Main St ".into(),
            line2: Some("  ".into()),
            locality: Some("Springfield".into()),
            region: None,
            postal_code: Some(" 12345 ".into()),
            country_code: "us".into(),
            is_primary: true,
        })
        .unwrap();
        assert_eq!(address.line1, "1 Main St");
        assert_eq!(address.line2, None);
        assert_eq!(address.postal_code.as_deref(), Some("12345"));
        assert_eq!(address.country_code, "US");

        let missing_line1 = AddressInput {
            label: "home".into(),
            country_code: "US".into(),
            ..AddressInput::default()
        };
        assert!(matches!(
            validate_address(missing_line1),
            Err(AuthError::InvalidProfileData(_))
        ));
    }
}
