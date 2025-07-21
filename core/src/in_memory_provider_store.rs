// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

use std::sync::Mutex;
use uuid::Uuid;

/// A non-optimized provider store implementation for use with tests
pub struct InMemoryProviderStore {
    users: Mutex<Vec<User>>,
    groups: Mutex<Vec<Group>>,
}

impl InMemoryProviderStore {
    pub fn new() -> Self {
        Self { users: Mutex::new(vec![]), groups: Mutex::new(vec![]) }
    }
}

#[async_trait]
impl ProviderStore for InMemoryProviderStore {
    async fn get_user_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<User>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|user| user.id == user_id).cloned())
    }

    async fn get_user_by_username(
        &self,
        user_name: String,
    ) -> Result<Option<User>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|user| user.name == user_name).cloned())
    }

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<User, ProviderStoreError> {
        if self.get_user_by_username(user_request.name.clone()).await?.is_some()
        {
            return Err(Error::conflict(user_request.name).into());
        }

        let new_user =
            User { id: Uuid::new_v4().to_string(), name: user_request.name };

        let mut users = self.users.lock().unwrap();
        users.push(new_user.clone());

        Ok(new_user)
    }

    async fn list_users(
        &self,
        _query_params: QueryParams,
    ) -> Result<Vec<User>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users.clone())
    }

    async fn get_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<Group>, ProviderStoreError> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.iter().find(|group| group.id == group_id).cloned())
    }

    async fn get_group_by_displayname(
        &self,
        display_name: String,
    ) -> Result<Option<Group>, ProviderStoreError> {
        let groups = self.groups.lock().unwrap();
        Ok(groups
            .iter()
            .find(|group| group.display_name == display_name)
            .cloned())
    }

    async fn create_group(
        &self,
        group_request: CreateGroupRequest,
    ) -> Result<Group, ProviderStoreError> {
        if self
            .get_group_by_displayname(group_request.display_name.clone())
            .await?
            .is_some()
        {
            return Err(Error::conflict(group_request.display_name).into());
        }

        let new_group = Group {
            id: Uuid::new_v4().to_string(),
            display_name: group_request.display_name,
        };

        let mut groups = self.groups.lock().unwrap();
        groups.push(new_group.clone());

        Ok(new_group)
    }

    async fn list_groups(
        &self,
        _query_params: QueryParams,
    ) -> Result<Vec<Group>, ProviderStoreError> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.clone())
    }
}
