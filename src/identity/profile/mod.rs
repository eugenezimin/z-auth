//! User profile — optional, labelled phone numbers and postal addresses
//! kept by a user.  Organization memberships live in
//! [`crate::identity::organization`].

pub mod model;
pub mod repository;
pub mod service;
pub mod service_impl;
pub mod validation;

pub use model::{Address, AddressInput, NewAddress, NewPhone, Phone, PhoneInput};
pub use repository::ProfileRepository;
pub use service::ProfileService;
pub use service_impl::ProfileServiceImpl;
pub use validation::{normalize_country_code, normalize_label, normalize_phone};
