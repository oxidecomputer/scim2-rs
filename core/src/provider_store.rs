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
    ) -> Result<Option<StoredParts<User>>, ProviderStoreError>;

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<StoredParts<User>, ProviderStoreError>;

    async fn list_users(
        &self,
        filter: Option<FilterOp>,
    ) -> Result<Vec<StoredParts<User>>, ProviderStoreError>;

    async fn replace_user(
        &self,
        user_id: String,
        user_request: CreateUserRequest,
    ) -> Result<StoredParts<User>, ProviderStoreError>;

    // A Some(User) is returned if the User existed prior to the delete,
    // otherwise None is returned.
    async fn delete_user_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<StoredParts<User>>, ProviderStoreError>;

    async fn get_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<StoredParts<Group>>, ProviderStoreError>;

    async fn create_group(
        &self,
        group_request: CreateGroupRequest,
    ) -> Result<StoredParts<Group>, ProviderStoreError>;

    async fn list_groups(
        &self,
        filter: Option<FilterOp>,
    ) -> Result<Vec<StoredParts<Group>>, ProviderStoreError>;

    async fn replace_group(
        &self,
        group_id: String,
        group_request: CreateGroupRequest,
    ) -> Result<StoredParts<Group>, ProviderStoreError>;

    // Delete a group, and all group memberships.
    //
    // A Some(Group) is returned if the Group existed prior to the delete,
    // otherwise None is returned.
    async fn delete_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<StoredParts<Group>>, ProviderStoreError>;

    /*
    async fn get_user_group_membership(
        &self,
        user_id: String,
    ) -> Result<Vec<UserGroup>, ProviderStoreError>;

    async fn get_group_members(
        &self,
        group_id: String,
    ) -> Result<Vec<GroupMember>, ProviderStoreError>;
    */
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
