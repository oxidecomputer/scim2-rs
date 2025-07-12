// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

use std::sync::Mutex;
use uuid::Uuid;

/// A non-optimized provider store implementation for use with tests
pub struct InMemoryProviderStore {
    users: Mutex<Vec<User>>,
}

impl InMemoryProviderStore {
    pub fn new() -> Self {
        let store = Self {
            users: Mutex::new(vec![]),
        };

        store
    }
}

#[async_trait]
impl ProviderStore for InMemoryProviderStore {
    async fn get_user_by_id(&self, user_id: String) -> Result<Option<User>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users
            .iter()
            .filter(|user| user.id == user_id)
            .next()
            .cloned())
    }

    async fn get_user_by_username(
        &self,
        user_name: String,
    ) -> Result<Option<User>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users
            .iter()
            .filter(|user| user.name == user_name)
            .next()
            .cloned())
    }

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<User, ProviderStoreError> {
        if self
            .get_user_by_username(user_request.name.clone())
            .await?
            .is_some()
        {
            return Err(Error::conflict(user_request.name).into());
        }

        let new_user = User {
            id: Uuid::new_v4().to_string(),
            name: user_request.name,
        };

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
}
