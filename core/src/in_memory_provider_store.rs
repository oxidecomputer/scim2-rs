// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

use std::sync::Mutex;
use uuid::Uuid;

/// A non-optimized provider store implementation for use with tests
pub struct InMemoryProviderStore {
    users: Mutex<Vec<StoredUser>>,
    groups: Mutex<Vec<StoredGroup>>,
}

impl Default for InMemoryProviderStore {
    fn default() -> Self {
        Self::new()
    }
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
    ) -> Result<Option<StoredUser>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|user| user.id == user_id).cloned())
    }

    async fn get_user_by_username(
        &self,
        user_name: String,
    ) -> Result<Option<StoredUser>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        Ok(users.iter().find(|user| user.name == user_name).cloned())
    }

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<StoredUser, ProviderStoreError> {
        if self.get_user_by_username(user_request.name.clone()).await?.is_some()
        {
            return Err(Error::conflict(user_request.name).into());
        }

        let new_user = StoredUser {
            id: Uuid::new_v4().to_string(),
            name: user_request.name,
            external_id: user_request.external_id,
            // If the client doesn't assert `active`, then default to true: if
            // an IdP doesn't use the `active` then we wouldn't want to have all
            // users they provision be disabled.
            active: user_request.active.unwrap_or(true),
            created: Utc::now(),
            last_modified: Utc::now(),
            version: String::from("W/unimplemented"),
        };

        let mut users = self.users.lock().unwrap();
        users.push(new_user.clone());

        Ok(new_user)
    }

    async fn list_users(
        &self,
        query_params: QueryParams,
    ) -> Result<Vec<StoredUser>, ProviderStoreError> {
        let users = self.users.lock().unwrap();
        let mut users = users.clone();
        if let Some(filter) = query_params.filter() {
            match filter {
                FilterOp::UserNameEq(username) => {
                    users.retain(|u| u.name.eq_ignore_ascii_case(&username))
                }
                _ => {
                    return Err(Error::invalid_filter(
                        "invalid or unsupported filter".to_string(),
                    )
                    .into());
                }
            }
        };
        Ok(users)
    }

    async fn replace_user(
        &self,
        user_id: String,
        user_request: CreateUserRequest,
    ) -> Result<StoredUser, ProviderStoreError> {
        let mut users = self.users.lock().unwrap();

        let index = match users.iter().position(|user| user.id == user_id) {
            None => {
                return Err(Error::not_found(user_id).into());
            }

            Some(index) => index,
        };

        // userName is meant to be unique. If the user request is changing the
        // username to one that already exists, then reject it.

        match users.iter().position(|user| {
            user.name == user_request.name && user.id != user_id
        }) {
            Some(_) => {
                return Err(Error::conflict(format!(
                    "username {}",
                    user_request.name
                ))
                .into());
            }

            None => {
                // ok
            }
        }

        // Update the modification time
        users[index].last_modified = Utc::now();

        // RFC 7664 § 3.5.1:
        // Attributes whose mutability is "readWrite" that are omitted from the
        // request body MAY be assumed to be not asserted by the client. The
        // service provider MAY assume that any existing values are to be
        // cleared, or the service provider MAY assign a default value to the
        // final resource representation.

        // This code takes the stance: if a provisioning client doesn't assert a
        // field, leave it alone: the IdP is the source of truth.

        let CreateUserRequest { name, active, external_id } = user_request;

        users[index].name = name;

        if let Some(active) = active {
            users[index].active = active;
        }

        if let Some(external_id) = external_id {
            users[index].external_id = Some(external_id);
        }

        Ok(users[index].clone())
    }

    async fn delete_user_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<StoredUser>, ProviderStoreError> {
        let mut users = self.users.lock().unwrap();
        let maybe_user = users.extract_if(.., |user| user.id == user_id).next();
        Ok(maybe_user)
    }

    async fn get_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<StoredGroup>, ProviderStoreError> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.iter().find(|group| group.id == group_id).cloned())
    }

    async fn get_group_by_displayname(
        &self,
        display_name: String,
    ) -> Result<Option<StoredGroup>, ProviderStoreError> {
        let groups = self.groups.lock().unwrap();
        Ok(groups
            .iter()
            .find(|group| group.display_name == display_name)
            .cloned())
    }

    async fn create_group(
        &self,
        group_request: CreateGroupRequest,
    ) -> Result<StoredGroup, ProviderStoreError> {
        let new_group = StoredGroup {
            id: Uuid::new_v4().to_string(),
            display_name: group_request.display_name,
            external_id: group_request.external_id,
            created: Utc::now(),
            last_modified: Utc::now(),
            version: String::from("W/unimplemented"),
        };

        let mut groups = self.groups.lock().unwrap();
        groups.push(new_group.clone());

        Ok(new_group)
    }

    async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<Vec<StoredGroup>, ProviderStoreError> {
        let groups = self.groups.lock().unwrap();
        let mut groups = groups.clone();
        if let Some(filter) = query_params.filter() {
            match filter {
                FilterOp::DisplayNameEq(display_name) => groups.retain(|g| {
                    g.display_name.eq_ignore_ascii_case(&display_name)
                }),
                _ => {
                    return Err(Error::invalid_filter(
                        "invalid or unsupported filter".to_string(),
                    )
                    .into());
                }
            }
        };
        Ok(groups)
    }

    async fn replace_group(
        &self,
        group_id: String,
        group_request: CreateGroupRequest,
    ) -> Result<StoredGroup, ProviderStoreError> {
        let mut groups = self.groups.lock().unwrap();

        let index = match groups.iter().position(|group| group.id == group_id) {
            None => {
                return Err(Error::not_found(group_id).into());
            }

            Some(index) => index,
        };

        // Update the modification time
        groups[index].last_modified = Utc::now();

        // RFC 7664 § 3.5.1:
        // Attributes whose mutability is "readWrite" that are omitted from the
        // request body MAY be assumed to be not asserted by the client. The
        // service provider MAY assume that any existing values are to be
        // cleared, or the service provider MAY assign a default value to the
        // final resource representation.

        let CreateGroupRequest { display_name, external_id } = group_request;

        groups[index].display_name = display_name;

        // This code takes the stance: if a provisioning client doesn't assert a
        // field, leave it alone: the IdP is the source of truth.

        if let Some(external_id) = external_id {
            groups[index].external_id = Some(external_id);
        }

        Ok(groups[index].clone())
    }

    async fn delete_group_by_id(
        &self,
        group_id: String,
    ) -> Result<Option<StoredGroup>, ProviderStoreError> {
        let mut groups = self.groups.lock().unwrap();
        let maybe_group =
            groups.extract_if(.., |group| group.id == group_id).next();
        Ok(maybe_group)
    }
}
