//! User profiles and organizations (feature `profile`):
//! [`PgProfileRepository`] and [`PgOrganizationRepository`].  Their tables
//! are created by [`PROFILE_MIGRATOR`](crate::PROFILE_MIGRATOR); their
//! statements live in [`tx::profiles`](crate::tx::profiles) and
//! [`tx::organizations`](crate::tx::organizations).

pub(crate) mod organization_queries;
mod organization_repository;
pub(crate) mod profile_queries;
mod profile_repository;

pub use organization_repository::PgOrganizationRepository;
pub use profile_repository::PgProfileRepository;
