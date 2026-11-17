-- -------------------------------------------------------------
-- Migration 0003 — the account_changed revocation reason
-- -------------------------------------------------------------
--
-- * revocation_reason gains account_changed: a host ends a user's
--   sessions because someone else changed the account, its roles,
--   permissions or placement, and the user signs in again to carry the
--   change.  Adding an enum value rewrites nothing.

ALTER TYPE revocation_reason ADD VALUE 'account_changed';
