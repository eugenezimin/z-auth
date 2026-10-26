//! SQL query constants for `identity_organizations`,
//! `identity_organization_departments` and
//! `identity_organization_memberships`.

macro_rules! organization_columns {
    () => {
        "id, name, description, created_at, updated_at"
    };
}

macro_rules! department_columns {
    () => {
        "id, organization_id, name, description, created_at, updated_at"
    };
}

/// A membership with its organization's and department's names, read from
/// `m` (the table or a CTE returning its rows).
macro_rules! membership_select {
    () => {
        "SELECT m.id, m.user_id, m.organization_id, o.name AS organization_name, \
                m.department_id, d.name AS department_name, m.title, m.is_primary, \
                m.created_at, m.updated_at \
         FROM m \
         JOIN identity_organizations o ON o.id = m.organization_id \
         LEFT JOIN identity_organization_departments d ON d.id = m.department_id"
    };
}

// ── Organizations ─────────────────────────────────────────────────────────────

pub const FIND_ORGANIZATION: &str = concat!(
    "SELECT ",
    organization_columns!(),
    " FROM identity_organizations WHERE id = $1"
);

pub const LIST_ORGANIZATIONS: &str = concat!(
    "SELECT ",
    organization_columns!(),
    " FROM identity_organizations \
     WHERE $1::text IS NULL OR strpos(lower(name), lower($1)) > 0 \
     ORDER BY lower(name), id \
     LIMIT $2 OFFSET $3"
);

/// Total for [`LIST_ORGANIZATIONS`] with the same `$1` filter.
pub const COUNT_ORGANIZATIONS: &str = "SELECT count(*) FROM identity_organizations \
     WHERE $1::text IS NULL OR strpos(lower(name), lower($1)) > 0";

pub const INSERT_ORGANIZATION: &str = concat!(
    "INSERT INTO identity_organizations (name, description) VALUES ($1, $2) RETURNING ",
    organization_columns!()
);

pub const UPDATE_ORGANIZATION: &str = concat!(
    "UPDATE identity_organizations SET name = $2, description = $3, updated_at = now() \
     WHERE id = $1 RETURNING ",
    organization_columns!()
);

pub const DELETE_ORGANIZATION: &str = "DELETE FROM identity_organizations WHERE id = $1";

// ── Departments ───────────────────────────────────────────────────────────────

pub const LIST_DEPARTMENTS: &str = concat!(
    "SELECT ",
    department_columns!(),
    " FROM identity_organization_departments WHERE organization_id = $1 \
     ORDER BY lower(name), id"
);

pub const FIND_DEPARTMENT: &str = concat!(
    "SELECT ",
    department_columns!(),
    " FROM identity_organization_departments WHERE id = $1"
);

pub const INSERT_DEPARTMENT: &str = concat!(
    "INSERT INTO identity_organization_departments (organization_id, name, description) \
     VALUES ($1, $2, $3) RETURNING ",
    department_columns!()
);

pub const UPDATE_DEPARTMENT: &str = concat!(
    "UPDATE identity_organization_departments \
     SET name = $2, description = $3, updated_at = now() WHERE id = $1 RETURNING ",
    department_columns!()
);

pub const DELETE_DEPARTMENT: &str = "DELETE FROM identity_organization_departments WHERE id = $1";

// ── Memberships ───────────────────────────────────────────────────────────────

pub const LIST_MEMBERSHIPS: &str = concat!(
    "WITH m AS (SELECT * FROM identity_organization_memberships WHERE user_id = $1) ",
    membership_select!(),
    " ORDER BY m.is_primary DESC, m.created_at, m.id"
);

/// `$2`, when set, keeps one department's members.
pub const LIST_MEMBERS: &str = concat!(
    "WITH m AS (SELECT * FROM identity_organization_memberships \
                WHERE organization_id = $1 AND ($2::uuid IS NULL OR department_id = $2)) ",
    membership_select!(),
    " ORDER BY m.created_at, m.id LIMIT $3 OFFSET $4"
);

/// Total for [`LIST_MEMBERS`] with the same `$1` / `$2` filter.
pub const COUNT_MEMBERS: &str = "SELECT count(*) FROM identity_organization_memberships \
     WHERE organization_id = $1 AND ($2::uuid IS NULL OR department_id = $2)";

/// A user's membership by id.
pub const FIND_MEMBERSHIP: &str = concat!(
    "WITH m AS (SELECT * FROM identity_organization_memberships \
                WHERE id = $1 AND user_id = $2) ",
    membership_select!()
);

/// Clears the user's primary membership, except `$2` when it is not NULL.
pub const CLEAR_PRIMARY_MEMBERSHIP: &str = "UPDATE identity_organization_memberships \
     SET is_primary = false, updated_at = now() \
     WHERE user_id = $1 AND is_primary AND ($2::uuid IS NULL OR id <> $2)";

pub const INSERT_MEMBERSHIP: &str = concat!(
    "WITH m AS (INSERT INTO identity_organization_memberships \
                (user_id, organization_id, department_id, title, is_primary) \
                VALUES ($1, $2, $3, $4, $5) RETURNING *) ",
    membership_select!()
);

/// Every field but the primary flag.
pub const UPDATE_MEMBERSHIP: &str = concat!(
    "WITH m AS (UPDATE identity_organization_memberships \
                SET organization_id = $3, department_id = $4, title = $5, updated_at = now() \
                WHERE id = $1 AND user_id = $2 RETURNING *) ",
    membership_select!()
);

pub const MEMBERSHIP_EXISTS: &str = "SELECT EXISTS (SELECT 1 FROM identity_organization_memberships \
     WHERE id = $1 AND user_id = $2)";

pub const SET_PRIMARY_MEMBERSHIP: &str = "UPDATE identity_organization_memberships \
     SET is_primary = true, updated_at = now() WHERE id = $1 AND user_id = $2";

pub const DELETE_MEMBERSHIP: &str =
    "DELETE FROM identity_organization_memberships WHERE id = $1 AND user_id = $2";
