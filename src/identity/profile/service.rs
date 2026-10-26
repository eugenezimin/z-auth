//! Profile service interface.
//!
//! Defines the [`ProfileService`] trait — a user's phone numbers and
//! addresses.  The default implementation is
//! [`ProfileServiceImpl`](crate::identity::profile::ProfileServiceImpl).

use async_trait::async_trait;
use uuid::Uuid;

use crate::identity::profile::model::{Address, AddressInput, Phone, PhoneInput};
use crate::utils::error::AuthError;

/// A user's optional phone numbers and postal addresses.
///
/// The first item of a kind a user adds becomes primary.  Deleting the
/// primary item does not promote another one.
#[async_trait]
pub trait ProfileService: Send + Sync {
    /// A user's phones, the primary first, then oldest first.
    async fn phones(&self, user_id: Uuid) -> Result<Vec<Phone>, AuthError>;

    /// Validate and add a phone.
    ///
    /// # Errors
    /// [`AuthError::InvalidPhoneNumber`], [`AuthError::InvalidProfileData`],
    /// [`AuthError::PhoneAlreadyAdded`], [`AuthError::ProfileLimitReached`],
    /// [`AuthError::UserNotFound`].
    async fn add_phone(&self, user_id: Uuid, input: PhoneInput) -> Result<Phone, AuthError>;

    /// Validate and replace a phone's label and number;
    /// [`AuthError::PhoneNotFound`] if the user has no such phone.
    async fn replace_phone(
        &self,
        user_id: Uuid,
        phone_id: Uuid,
        input: PhoneInput,
    ) -> Result<Phone, AuthError>;

    /// Make a phone the user's primary one.
    async fn set_primary_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<(), AuthError>;

    async fn delete_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<(), AuthError>;

    /// A user's addresses, the primary first, then oldest first.
    async fn addresses(&self, user_id: Uuid) -> Result<Vec<Address>, AuthError>;

    /// Validate and add an address.
    ///
    /// # Errors
    /// [`AuthError::InvalidProfileData`], [`AuthError::ProfileLimitReached`],
    /// [`AuthError::UserNotFound`].
    async fn add_address(&self, user_id: Uuid, input: AddressInput) -> Result<Address, AuthError>;

    /// Validate and replace every field of an address;
    /// [`AuthError::AddressNotFound`] if the user has no such address.
    async fn replace_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        input: AddressInput,
    ) -> Result<Address, AuthError>;

    /// Make an address the user's primary one.
    async fn set_primary_address(&self, user_id: Uuid, address_id: Uuid) -> Result<(), AuthError>;

    async fn delete_address(&self, user_id: Uuid, address_id: Uuid) -> Result<(), AuthError>;
}
