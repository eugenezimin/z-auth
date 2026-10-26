//! Profile service implementation.
//!
//! [`ProfileServiceImpl`] validates and normalizes every input, enforces the
//! per-user limits and makes a user's first item of a kind primary, then
//! hands the result to the [`ProfileRepository`] port.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::conf::constants::{MAX_ADDRESSES_PER_USER, MAX_PHONES_PER_USER};
use crate::identity::profile::model::{Address, AddressInput, Phone, PhoneInput};
use crate::identity::profile::repository::ProfileRepository;
use crate::identity::profile::service::ProfileService;
use crate::identity::profile::validation::{validate_address, validate_phone};
use crate::utils::error::AuthError;

/// Default implementation of [`ProfileService`].
pub struct ProfileServiceImpl {
    repo: Option<Arc<dyn ProfileRepository>>,
}

impl ProfileServiceImpl {
    /// Without `repo` every call fails with [`AuthError::Config`].
    pub fn new(repo: Option<Arc<dyn ProfileRepository>>) -> Self {
        Self { repo }
    }

    fn repo(&self) -> Result<&dyn ProfileRepository, AuthError> {
        self.repo.as_deref().ok_or_else(|| {
            AuthError::Config("missing adapter: user profiles (ZAuthBuilder::profiles)".into())
        })
    }
}

fn limit_reached(what: &str, max: usize) -> AuthError {
    AuthError::ProfileLimitReached(format!("a user may keep at most {max} {what}"))
}

#[async_trait]
impl ProfileService for ProfileServiceImpl {
    async fn phones(&self, user_id: Uuid) -> Result<Vec<Phone>, AuthError> {
        self.repo()?.list_phones(user_id).await
    }

    async fn add_phone(&self, user_id: Uuid, input: PhoneInput) -> Result<Phone, AuthError> {
        let repo = self.repo()?;
        let mut phone = validate_phone(input)?;
        let existing = repo.list_phones(user_id).await?;
        if existing.len() >= MAX_PHONES_PER_USER {
            return Err(limit_reached("phone numbers", MAX_PHONES_PER_USER));
        }
        phone.is_primary |= existing.is_empty();
        repo.add_phone(user_id, phone).await
    }

    async fn replace_phone(
        &self,
        user_id: Uuid,
        phone_id: Uuid,
        input: PhoneInput,
    ) -> Result<Phone, AuthError> {
        let phone = validate_phone(input)?;
        self.repo()?
            .replace_phone(user_id, phone_id, phone)
            .await?
            .ok_or(AuthError::PhoneNotFound)
    }

    async fn set_primary_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<(), AuthError> {
        match self.repo()?.set_primary_phone(user_id, phone_id).await? {
            true => Ok(()),
            false => Err(AuthError::PhoneNotFound),
        }
    }

    async fn delete_phone(&self, user_id: Uuid, phone_id: Uuid) -> Result<(), AuthError> {
        match self.repo()?.delete_phone(user_id, phone_id).await? {
            true => Ok(()),
            false => Err(AuthError::PhoneNotFound),
        }
    }

    async fn addresses(&self, user_id: Uuid) -> Result<Vec<Address>, AuthError> {
        self.repo()?.list_addresses(user_id).await
    }

    async fn add_address(&self, user_id: Uuid, input: AddressInput) -> Result<Address, AuthError> {
        let repo = self.repo()?;
        let mut address = validate_address(input)?;
        let existing = repo.list_addresses(user_id).await?;
        if existing.len() >= MAX_ADDRESSES_PER_USER {
            return Err(limit_reached("addresses", MAX_ADDRESSES_PER_USER));
        }
        address.is_primary |= existing.is_empty();
        repo.add_address(user_id, address).await
    }

    async fn replace_address(
        &self,
        user_id: Uuid,
        address_id: Uuid,
        input: AddressInput,
    ) -> Result<Address, AuthError> {
        let address = validate_address(input)?;
        self.repo()?
            .replace_address(user_id, address_id, address)
            .await?
            .ok_or(AuthError::AddressNotFound)
    }

    async fn set_primary_address(&self, user_id: Uuid, address_id: Uuid) -> Result<(), AuthError> {
        match self
            .repo()?
            .set_primary_address(user_id, address_id)
            .await?
        {
            true => Ok(()),
            false => Err(AuthError::AddressNotFound),
        }
    }

    async fn delete_address(&self, user_id: Uuid, address_id: Uuid) -> Result<(), AuthError> {
        match self.repo()?.delete_address(user_id, address_id).await? {
            true => Ok(()),
            false => Err(AuthError::AddressNotFound),
        }
    }
}
