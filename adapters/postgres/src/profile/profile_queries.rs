//! SQL query constants for `identity_user_phones` and
//! `identity_user_addresses`.

macro_rules! phone_columns {
    () => {
        "id, user_id, label, number, is_primary, created_at, updated_at"
    };
}

macro_rules! address_columns {
    () => {
        "id, user_id, label, line1, line2, locality, region, postal_code, country_code, \
         is_primary, created_at, updated_at"
    };
}

/// Serializes primary changes for one user (see the migration's header).
/// `FOR NO KEY UPDATE` does not block the `KEY SHARE` locks foreign keys
/// take.  No row: the user does not exist.
pub const LOCK_USER: &str = "SELECT id FROM identity_users WHERE id = $1 FOR NO KEY UPDATE";

// ── Phones ────────────────────────────────────────────────────────────────────

pub const LIST_PHONES: &str = concat!(
    "SELECT ",
    phone_columns!(),
    " FROM identity_user_phones WHERE user_id = $1 \
     ORDER BY is_primary DESC, created_at, id"
);

pub const CLEAR_PRIMARY_PHONE: &str = "UPDATE identity_user_phones \
     SET is_primary = false, updated_at = now() WHERE user_id = $1 AND is_primary";

pub const INSERT_PHONE: &str = concat!(
    "INSERT INTO identity_user_phones (user_id, label, number, is_primary) \
     VALUES ($1, $2, $3, $4) RETURNING ",
    phone_columns!()
);

/// `$5 = false` keeps the phone's primary flag.
pub const REPLACE_PHONE: &str = concat!(
    "UPDATE identity_user_phones \
     SET label = $3, number = $4, is_primary = is_primary OR $5, updated_at = now() \
     WHERE id = $1 AND user_id = $2 RETURNING ",
    phone_columns!()
);

pub const SET_PRIMARY_PHONE: &str = "UPDATE identity_user_phones \
     SET is_primary = true, updated_at = now() WHERE id = $1 AND user_id = $2";

pub const DELETE_PHONE: &str = "DELETE FROM identity_user_phones WHERE id = $1 AND user_id = $2";

// ── Addresses ─────────────────────────────────────────────────────────────────

pub const LIST_ADDRESSES: &str = concat!(
    "SELECT ",
    address_columns!(),
    " FROM identity_user_addresses WHERE user_id = $1 \
     ORDER BY is_primary DESC, created_at, id"
);

pub const CLEAR_PRIMARY_ADDRESS: &str = "UPDATE identity_user_addresses \
     SET is_primary = false, updated_at = now() WHERE user_id = $1 AND is_primary";

pub const INSERT_ADDRESS: &str = concat!(
    "INSERT INTO identity_user_addresses \
     (user_id, label, line1, line2, locality, region, postal_code, country_code, is_primary) \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING ",
    address_columns!()
);

/// `$10 = false` keeps the address's primary flag.
pub const REPLACE_ADDRESS: &str = concat!(
    "UPDATE identity_user_addresses \
     SET label = $3, line1 = $4, line2 = $5, locality = $6, region = $7, postal_code = $8, \
         country_code = $9, is_primary = is_primary OR $10, updated_at = now() \
     WHERE id = $1 AND user_id = $2 RETURNING ",
    address_columns!()
);

pub const SET_PRIMARY_ADDRESS: &str = "UPDATE identity_user_addresses \
     SET is_primary = true, updated_at = now() WHERE id = $1 AND user_id = $2";

pub const DELETE_ADDRESS: &str =
    "DELETE FROM identity_user_addresses WHERE id = $1 AND user_id = $2";
