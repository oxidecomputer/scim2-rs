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
    ) -> Result<Option<StoredUser>, ProviderStoreError>;

    async fn get_user_by_username(
        &self,
        user_name: String,
    ) -> Result<Option<StoredUser>, ProviderStoreError>;

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<StoredUser, ProviderStoreError>;

    async fn list_users(
        &self,
        query_params: QueryParams,
    ) -> Result<Vec<StoredUser>, ProviderStoreError>;

    async fn replace_user(
        &self,
        user_id: String,
        user_request: CreateUserRequest,
    ) -> Result<StoredUser, ProviderStoreError>;

    // A Some(StoredUser) is returned if the User existed prior to the delete,
    // otherwise None is returned.
    async fn delete_user_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<StoredUser>, ProviderStoreError>;

    async fn get_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<StoredGroup>, ProviderStoreError>;

    async fn get_group_by_displayname(
        &self,
        display_name: String,
    ) -> Result<Option<StoredGroup>, ProviderStoreError>;

    async fn create_group_with_members(
        &self,
        group_request: CreateGroupRequest,
        members: Vec<StoredGroupMember>,
    ) -> Result<StoredGroup, ProviderStoreError>;

    async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<Vec<StoredGroup>, ProviderStoreError>;

    async fn replace_group_with_members(
        &self,
        group_id: String,
        group_request: CreateGroupRequest,
        members: Vec<StoredGroupMember>,
    ) -> Result<StoredGroup, ProviderStoreError>;

    // Delete a group, and all group memberships.
    //
    // A Some(StoredGroup) is returned if the Group existed prior to the delete,
    // otherwise None is returned.
    async fn delete_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<StoredGroup>, ProviderStoreError>;

    async fn get_user_group_membership(
        &self,
        user_id: String,
    ) -> Result<Vec<UserGroup>, ProviderStoreError>;

    async fn get_group_members(
        &self,
        group_id: String,
    ) -> Result<Vec<StoredGroupMember>, ProviderStoreError>;
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
