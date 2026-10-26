//! Identity contexts — [`user`] accounts, their [`credentials`] (password
//! hashing and policy), [`access`] (roles and grants), [`external`]
//! identities linked to them, and optional [`profile`] data (phones,
//! addresses) and [`organization`] memberships.

pub mod access;
pub mod credentials;
pub mod external;
pub mod organization;
pub mod profile;
pub mod user;
