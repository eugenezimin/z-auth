-- -------------------------------------------------------------
-- Author: Eugene Zimin
-- Database: z_auth
-- Migration 4001 — user profiles and organizations
--                  (feature `profile`)
-- -------------------------------------------------------------
--
-- Applied by `z_auth_postgres::PROFILE_MIGRATOR`, after the core set
-- (`migrations/`).  Versions start at 4001 so they never collide with the
-- core (< 1000), cluster (1001–1999), mfa (2001–2999) or external-identity
-- (3001–3999) sets, which share `_sqlx_migrations`.  Never edit this file
-- once applied anywhere — add `4002_<description>.sql` instead.
--
-- Needs PostgreSQL 15 or later (`UNIQUE NULLS NOT DISTINCT`, `ON DELETE
-- SET NULL (column)`).
--
-- Everything here is optional data a user may enter; none of it is read
-- at login.  No table is in the cluster change feed: profiles are kept at
-- the authority only.
--
-- At most one primary phone, address and membership per user is enforced
-- by partial unique indexes; the adapter changes the primary in one
-- transaction that first locks the user's row, so concurrent changes for
-- one user serialize instead of failing.

-- ── identity_user_phones ──────────────────────────────────────────────────────
-- A user's phone numbers.  `number` is E.164 (normalized by z-auth before
-- it reaches the database); `label` is free text chosen by the user.
CREATE TABLE "identity_user_phones" (
    "id"         uuid        NOT NULL DEFAULT gen_random_uuid(),
    "user_id"    uuid        NOT NULL,
    "label"      varchar(32) NOT NULL,
    "number"     varchar(16) NOT NULL,
    "is_primary" bool        NOT NULL DEFAULT false,
    "created_at" timestamptz NOT NULL DEFAULT now(),
    "updated_at" timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    CONSTRAINT fk_identity_user_phones_user_id
        FOREIGN KEY ("user_id") REFERENCES "identity_users" ("id") ON DELETE CASCADE,
    CONSTRAINT uq_identity_user_phones_number UNIQUE ("user_id", "number"),
    CONSTRAINT chk_identity_user_phones_label
        CHECK (char_length(btrim("label")) BETWEEN 1 AND 32),
    CONSTRAINT chk_identity_user_phones_number
        CHECK ("number" ~ '^\+[1-9][0-9]{6,14}$')
);

-- uq_identity_user_phones_number leads with user_id: it also serves the
-- per-user listing and the foreign key.
CREATE UNIQUE INDEX uq_identity_user_phones_primary
    ON identity_user_phones USING btree (user_id)
    WHERE is_primary;

-- ── identity_user_addresses ───────────────────────────────────────────────────
-- A user's postal addresses.  `country_code` is ISO 3166-1 alpha-2;
-- `locality` is the city or town, `region` the state or province.
CREATE TABLE "identity_user_addresses" (
    "id"           uuid         NOT NULL DEFAULT gen_random_uuid(),
    "user_id"      uuid         NOT NULL,
    "label"        varchar(32)  NOT NULL,
    "line1"        varchar(255) NOT NULL,
    "line2"        varchar(255),
    "locality"     varchar(255),
    "region"       varchar(255),
    "postal_code"  varchar(20),
    "country_code" char(2)      NOT NULL,
    "is_primary"   bool         NOT NULL DEFAULT false,
    "created_at"   timestamptz  NOT NULL DEFAULT now(),
    "updated_at"   timestamptz  NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    CONSTRAINT fk_identity_user_addresses_user_id
        FOREIGN KEY ("user_id") REFERENCES "identity_users" ("id") ON DELETE CASCADE,
    CONSTRAINT chk_identity_user_addresses_label
        CHECK (char_length(btrim("label")) BETWEEN 1 AND 32),
    CONSTRAINT chk_identity_user_addresses_line1
        CHECK (char_length(btrim("line1")) > 0),
    CONSTRAINT chk_identity_user_addresses_country_code
        CHECK ("country_code" ~ '^[A-Z]{2}$')
);

CREATE INDEX idx_identity_user_addresses_user
    ON identity_user_addresses USING btree (user_id, created_at);

CREATE UNIQUE INDEX uq_identity_user_addresses_primary
    ON identity_user_addresses USING btree (user_id)
    WHERE is_primary;

-- ── identity_organizations ────────────────────────────────────────────────────
-- The shared catalog of organizations users may belong to.  Names are
-- unique case-insensitively and keep their display case.
CREATE TABLE "identity_organizations" (
    "id"          uuid         NOT NULL DEFAULT gen_random_uuid(),
    "name"        varchar(255) NOT NULL,
    "description" text,
    "created_at"  timestamptz  NOT NULL DEFAULT now(),
    "updated_at"  timestamptz  NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    CONSTRAINT chk_identity_organizations_name
        CHECK (char_length(btrim("name")) > 0)
);

CREATE UNIQUE INDEX identity_organizations_name_key
    ON identity_organizations USING btree (lower(name));

-- ── identity_organization_departments ─────────────────────────────────────────
-- Departments of an organization; deleted with it.  Names are unique
-- within the organization, case-insensitively.  (organization_id, id) is
-- unique only so that memberships can reference both columns.
CREATE TABLE "identity_organization_departments" (
    "id"              uuid         NOT NULL DEFAULT gen_random_uuid(),
    "organization_id" uuid         NOT NULL,
    "name"            varchar(255) NOT NULL,
    "description"     text,
    "created_at"      timestamptz  NOT NULL DEFAULT now(),
    "updated_at"      timestamptz  NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    CONSTRAINT fk_identity_organization_departments_organization_id
        FOREIGN KEY ("organization_id") REFERENCES "identity_organizations" ("id")
        ON DELETE CASCADE,
    CONSTRAINT uq_identity_organization_departments_organization_id
        UNIQUE ("organization_id", "id"),
    CONSTRAINT chk_identity_organization_departments_name
        CHECK (char_length(btrim("name")) > 0)
);

-- Leads with organization_id: also serves the per-organization listing and
-- the foreign key.
CREATE UNIQUE INDEX identity_organization_departments_name_key
    ON identity_organization_departments USING btree (organization_id, lower(name));

-- ── identity_organization_memberships ─────────────────────────────────────────
-- A user's membership in an organization, optionally in one of its
-- departments, with a job title.  The composite foreign key guarantees the
-- department belongs to the same organization; deleting the department
-- only clears `department_id`.  A user holds each (organization,
-- department) pair once, "no department" included (NULLS NOT DISTINCT).
CREATE TABLE "identity_organization_memberships" (
    "id"              uuid         NOT NULL DEFAULT gen_random_uuid(),
    "user_id"         uuid         NOT NULL,
    "organization_id" uuid         NOT NULL,
    "department_id"   uuid,
    "title"           varchar(255),
    "is_primary"      bool         NOT NULL DEFAULT false,
    "created_at"      timestamptz  NOT NULL DEFAULT now(),
    "updated_at"      timestamptz  NOT NULL DEFAULT now(),
    PRIMARY KEY ("id"),
    CONSTRAINT fk_identity_organization_memberships_user_id
        FOREIGN KEY ("user_id") REFERENCES "identity_users" ("id") ON DELETE CASCADE,
    CONSTRAINT fk_identity_organization_memberships_organization_id
        FOREIGN KEY ("organization_id") REFERENCES "identity_organizations" ("id")
        ON DELETE CASCADE,
    CONSTRAINT fk_identity_organization_memberships_department
        FOREIGN KEY ("organization_id", "department_id")
        REFERENCES "identity_organization_departments" ("organization_id", "id")
        ON DELETE SET NULL ("department_id"),
    CONSTRAINT uq_identity_organization_memberships_member
        UNIQUE NULLS NOT DISTINCT ("user_id", "organization_id", "department_id")
);

-- uq_identity_organization_memberships_member leads with user_id: it also
-- serves the per-user listing and the user foreign key.

-- Members of an organization (or one department), oldest first; also
-- serves both organization foreign keys.
CREATE INDEX idx_identity_organization_memberships_organization
    ON identity_organization_memberships USING btree (organization_id, department_id, created_at);

CREATE UNIQUE INDEX uq_identity_organization_memberships_primary
    ON identity_organization_memberships USING btree (user_id)
    WHERE is_primary;
