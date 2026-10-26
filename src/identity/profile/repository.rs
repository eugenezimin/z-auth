//! Profile repository port.

use async_trait::async_trait;
use uuid::Uuid;

use crate::identity::profile::model::{Address, NewAddress, NewPhone, Phone};
use crate::utils::error::AuthError;

/// Persistence contract for users' phone numbers and addresses.
///
/// - Every call on one item is scoped by `(user_id, id)`: an item of another
///   user is "not found".
/// - Writing an item with `is_primary = true` clears the user's previous
///   primary item of that kind in the same atomic step; at most one phone
///   and one address per user are primary.  `is_primary = false` on a
///   replace leaves the item's flag unchanged.
/// - Deleting a user must delete their phones and addresses.
///
/// Map driver errors to [`AuthError::Storage`], an unknown user to
/// [`AuthError::UserNotFound`], and a second phone with the same number
/// for one user to [`AuthError::PhoneAlreadyAdded`].
#[async_trait]
pub trait ProfileRepository: Send + Sync {
    /// A user's phones, the primary first, then oldest first.
    async fn list_phones(&self, user_id: Uuid) -> Result<Vec<Phone>, AuthError>;

    /// Insert a phone; `created_at` comes from the store.
    async fn add_phone(&self, user_id: Uuid, phone: NewPhone) -> Result<Phone, AuthError>;

    /// Replace a phone's label and number.  `Ok(None)` if the user has no
    /// such phone.
    async fn replace_phone(
        &self,
        user_id: Uuid,
        phone_id: Uuid,
        phone: NewPhone,
    ) -> Result<Option<Phone>, AuthError>;

    /// Make a phone the user's primary one.  Returns `false` if the user
    /// has no such phone.
    async fn set_primary_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<bool, AuthError>;

    /// Returns `false` if the user has no such phone.
    async fn delete_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<bool, AuthError>;

    /// A user's addresses, the primary first, then oldest first.
    async fn list_addresses(&self, user_id: Uuid) -> Result<Vec<Address>, AuthError>;

    /// Insert an address; `created_at` comes from the store.
    async fn add_address(&self, user_id: Uuid, address: NewAddress) -> Result<Address, AuthError>;

    /// Replace every field of an address.  `Ok(None)` if the user has no
    /// such address.
    async fn replace_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        address: NewAddress,
    ) -> Result<Option<Address>, AuthError>;

    /// Make an address the user's primary one.  Returns `false` if the
    /// user has no such address.
    async fn set_primary_address(&self, user_id: Uuid, address_id: Uuid)
    -> Result<bool, AuthError>;

    /// Returns `false` if the user has no such address.
    async fn delete_address(&self, user_id: Uuid, address_id: Uuid) -> Result<bool, AuthError>;
}
