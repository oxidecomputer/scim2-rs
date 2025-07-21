// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

/// The durable store for users and groups
#[async_trait]
pub trait ProviderStore: Sync {
    async fn get_user_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<User>, ProviderStoreError>;

    async fn get_user_by_username(
        &self,
        user_name: String,
    ) -> Result<Option<User>, ProviderStoreError>;

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<User, ProviderStoreError>;

    async fn list_users(
        &self,
        query_params: QueryParams,
    ) -> Result<Vec<User>, ProviderStoreError>;

    async fn get_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<Group>, ProviderStoreError>;

    async fn get_group_by_displayname(
        &self,
        display_name: String,
    ) -> Result<Option<Group>, ProviderStoreError>;

    async fn create_group(
        &self,
        group_request: CreateGroupRequest,
    ) -> Result<Group, ProviderStoreError>;

    async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<Vec<Group>, ProviderStoreError>;
}

/// The backing store for users and groups may throw its own error type, or it
/// may throw a SCIM error.
#[derive(Debug)]
pub enum ProviderStoreError {
    StoreError(anyhow::Error),
    Scim(Error),
}

impl From<Error> for ProviderStoreError {
    fn from(e: Error) -> ProviderStoreError {
        ProviderStoreError::Scim(e)
    }
}
impl From<anyhow::Error> for ProviderStoreError {
    fn from(e: anyhow::Error) -> ProviderStoreError {
        ProviderStoreError::StoreError(e)
    }
}
