// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Serialize, JsonSchema)]
pub struct InMemoryProviderStoreState {
    users: BTreeMap<String, StoredParts<User>>,
    groups: BTreeMap<String, StoredParts<Group>>,
}

impl InMemoryProviderStoreState {
    fn get_group_member(
        &self,
        member: &GroupMember,
    ) -> Result<GroupMember, Error> {
        let GroupMember { resource_type, value } = member;

        let Some(value) = value else {
            // The minimum that this code needs is the value field so complain
            // about that.
            return Err(Error::invalid_syntax(String::from(
                "group member missing value field",
            )));
        };

        // Find the ID that this request is talking about, or 404
        let resource_type = if let Some(resource_type) = resource_type {
            let resource_type = ResourceType::from_str(resource_type)
                .map_err(Error::invalid_syntax)?;

            match resource_type {
                ResourceType::User => {
                    self.users
                        .get(value)
                        .ok_or(Error::not_found(value.clone()))?;
                }

                ResourceType::Group => {
                    // don't support nested groups for now.
                    return Err(Error::internal_error(
                        "nested groups not supported".to_string(),
                    ));
                }
            }

            resource_type
        } else {
            let maybe_user = self.users.get(value);
            let maybe_group = self.groups.get(value);

            match (maybe_user, maybe_group) {
                (None, None) => {
                    // 404
                    return Err(Error::not_found(value.clone()));
                }

                (Some(_), None) => ResourceType::User,

                (None, Some(_)) => {
                    return Err(Error::internal_error(
                        "nested groups not supported".to_string(),
                    ));
                }

                (Some(_), Some(_)) => {
                    return Err(Error::internal_error(format!(
                        "{value} returned a user and group!"
                    )));
                }
            }
        };

        Ok(GroupMember {
            resource_type: Some(resource_type.to_string()),
            value: Some(value.clone()),
        })
    }
}

/// A non-optimized provider store implementation for use with tests
pub struct InMemoryProviderStore {
    state: Mutex<InMemoryProviderStoreState>,
}

impl Default for InMemoryProviderStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryProviderStore {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(InMemoryProviderStoreState {
                users: BTreeMap::new(),
                groups: BTreeMap::new(),
            }),
        }
    }

    pub fn state(&self) -> InMemoryProviderStoreState {
        self.state.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProviderStore for InMemoryProviderStore {
    async fn get_user_by_id(
        &self,
        user_id: &str,
    ) -> Result<Option<StoredParts<User>>, ProviderStoreError> {
        let state = self.state.lock().unwrap();
        Ok(state.users.get(user_id).cloned())
    }

    async fn create_user(
        &self,
        user_request: CreateUserRequest,
    ) -> Result<StoredParts<User>, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();

        if state
            .users
            .values()
            .any(|stored_part| stored_part.resource.name == user_request.name)
        {
            return Err(Error::conflict(user_request.name).into());
        }

        let id = Uuid::new_v4().to_string();

        let new_user = StoredParts {
            resource: User {
                id: id.clone(),
                name: user_request.name,
                external_id: user_request.external_id,
                active: user_request.active,
                groups: None,
            },

            meta: StoredMeta {
                created: Utc::now(),
                last_modified: Utc::now(),
                version: String::from("W/unimplemented"),
            },
        };

        let existing = state.users.insert(id, new_user.clone());
        assert!(existing.is_none());

        Ok(new_user)
    }

    async fn list_users(
        &self,
        filter: Option<FilterOp>,
    ) -> Result<Vec<StoredParts<User>>, ProviderStoreError> {
        let state = self.state.lock().unwrap();
        let mut users = state.users.clone();

        match filter {
            Some(FilterOp::UserNameEq(username)) => {
                users.retain(|_, stored_part| {
                    stored_part.resource.name.eq_ignore_ascii_case(&username)
                })
            }

            None => {
                // ok
            }

            Some(_) => {
                return Err(Error::invalid_filter(
                    "invalid or unsupported filter".to_string(),
                )
                .into());
            }
        }

        Ok(users.values().cloned().collect())
    }

    async fn replace_user(
        &self,
        user_id: &str,
        user_request: CreateUserRequest,
    ) -> Result<StoredParts<User>, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();
        let users = &mut state.users;

        // userName is meant to be unique. If the user request is changing the
        // username to one that already exists, then reject it.

        if users.values().any(|stored_part| {
            stored_part.resource.name == user_request.name
                && stored_part.resource.id != user_id
        }) {
            return Err(Error::conflict(format!(
                "username {}",
                user_request.name
            ))
            .into());
        }

        // Can't replace a user that does not exist, so return 404 if it's not
        // found
        let existing_user = users
            .get_mut(user_id)
            .ok_or(Error::not_found(user_id.to_string()))?;

        // RFC 7664 § 3.5.1:
        // Attributes whose mutability is "readWrite" that are omitted from the
        // request body MAY be assumed to be not asserted by the client. The
        // service provider MAY assume that any existing values are to be
        // cleared, or the service provider MAY assign a default value to the
        // final resource representation.

        // This code takes the stance: if a provisioning client doesn't assert a
        // field, leave it alone: the IdP is the source of truth. We simply
        // accept the request and write the fields in.

        *existing_user = StoredParts {
            resource: User {
                id: user_id.to_string(),
                name: user_request.name,
                external_id: user_request.external_id,
                active: user_request.active,
                groups: existing_user.resource.groups.clone(),
            },

            meta: StoredMeta {
                // Keep creation time
                created: existing_user.meta.created,
                // Update the modification time
                last_modified: Utc::now(),
                // Don't touch the version, this impl does not support that!
                version: existing_user.meta.version.clone(),
            },
        };

        Ok(existing_user.clone())
    }

    async fn delete_user_by_id(
        &self,
        user_id: &str,
    ) -> Result<bool, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();
        let maybe_user = state.users.remove(user_id);
        Ok(maybe_user.is_some())
    }

    async fn get_group_by_id(
        &self,
        group_id: &str,
    ) -> Result<Option<StoredParts<Group>>, ProviderStoreError> {
        let state = self.state.lock().unwrap();
        Ok(state.groups.get(group_id).cloned())
    }

    async fn create_group(
        &self,
        group_request: CreateGroupRequest,
    ) -> Result<StoredParts<Group>, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();

        // Make sure that display name is unique
        if state.groups.values().any(|stored_part| {
            stored_part.resource.display_name == group_request.display_name
        }) {
            return Err(Error::conflict(format!(
                "displayName {}",
                group_request.display_name
            ))
            .into());
        }

        let CreateGroupRequest { display_name, external_id, mut members } =
            group_request;

        let id = Uuid::new_v4().to_string();

        // Validate the members arg, and return filled in fields. Then fill in
        // the appropriate User's groups field.
        if let Some(members) = &mut members {
            for member in members {
                *member = state.get_group_member(member)?;

                // value will be filled in, so we can unwrap here
                let user_id: &String = member
                    .value
                    .as_ref()
                    .expect("get_group_member should have filled this in");

                // Add to the User's groups field
                state
                    .users
                    .get_mut(user_id)
                    .expect("get_group_member would returned 404 if the user didn't exist")
                    .resource
                    .groups
                    .get_or_insert_default()
                    .push(
                        UserGroup {
                            member_type: Some(UserGroupType::Direct),
                            value: Some(id.clone()),
                            display: Some(display_name.clone()),
                        }
                    );
            }
        }

        let new_group = StoredParts {
            resource: Group {
                id: id.clone(),
                display_name,
                external_id,
                members,
            },
            meta: StoredMeta {
                created: Utc::now(),
                last_modified: Utc::now(),
                version: String::from("W/unimplemented"),
            },
        };

        let existing = state.groups.insert(id, new_group.clone());
        assert!(existing.is_none());

        Ok(new_group)
    }

    async fn list_groups(
        &self,
        filter: Option<FilterOp>,
    ) -> Result<Vec<StoredParts<Group>>, ProviderStoreError> {
        let state = self.state.lock().unwrap();
        let mut groups = state.groups.clone();

        match filter {
            Some(FilterOp::DisplayNameEq(display_name)) => {
                groups.retain(|_, stored_part| {
                    stored_part
                        .resource
                        .display_name
                        .eq_ignore_ascii_case(&display_name)
                })
            }

            None => {
                // ok
            }

            Some(_) => {
                return Err(Error::invalid_filter(
                    "invalid or unsupported filter".to_string(),
                )
                .into());
            }
        }

        Ok(groups.values().cloned().collect())
    }

    async fn replace_group(
        &self,
        group_id: &str,
        group_request: CreateGroupRequest,
    ) -> Result<StoredParts<Group>, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();

        let CreateGroupRequest { display_name, external_id, mut members } =
            group_request;

        // Make sure that display name is unique
        if state.groups.values().any(|stored_part| {
            stored_part.resource.display_name == display_name
                && stored_part.resource.id != group_id
        }) {
            return Err(
                Error::conflict(format!("displayName {display_name}")).into()
            );
        }

        // Delete all existing group membership for this group id
        for stored_part in state.users.values_mut() {
            if let Some(groups) = &mut stored_part.resource.groups {
                groups.retain(|user_group| {
                    user_group.value.as_deref() != Some(group_id)
                });
            }
        }

        // Validate the members arg, and return filled in fields. Then fill in
        // the appropriate User's groups field.

        if let Some(members) = &mut members {
            for member in members {
                *member = state.get_group_member(member)?;

                // value will be filled in, so we can unwrap here
                let user_id: &String = member
                    .value
                    .as_ref()
                    .expect("get_group_member should have filled this in");

                // Add to the User's groups field
                state
                    .users
                    .get_mut(user_id)
                    .expect("get_group_member would returned 404 if the user didn't exist")
                    .resource
                    .groups
                    .get_or_insert_default()
                    .push(
                        UserGroup {
                            member_type: Some(UserGroupType::Direct),
                            value: Some(group_id.to_string()),
                            display: Some(display_name.clone()),
                        }
                    );
            }
        }

        // Can't replace a group that does not exist, so return 404 if it's not
        // found
        let existing_group = state
            .groups
            .get_mut(group_id)
            .ok_or(Error::not_found(group_id.to_string()))?;

        // RFC 7664 § 3.5.1:
        // Attributes whose mutability is "readWrite" that are omitted from the
        // request body MAY be assumed to be not asserted by the client. The
        // service provider MAY assume that any existing values are to be
        // cleared, or the service provider MAY assign a default value to the
        // final resource representation.

        // This code takes the stance: if a provisioning client doesn't assert a
        // field, leave it alone: the IdP is the source of truth. We simply
        // accept the request and write the fields in.

        *existing_group = StoredParts {
            resource: Group {
                id: group_id.to_string(),
                display_name,
                external_id,
                members,
            },

            meta: StoredMeta {
                // Keep creation time
                created: existing_group.meta.created,
                // Update the modification time
                last_modified: Utc::now(),
                // Don't touch the version, this impl does not support that!
                version: existing_group.meta.version.clone(),
            },
        };

        Ok(existing_group.clone())
    }

    async fn delete_group_by_id(
        &self,
        group_id: &str,
    ) -> Result<bool, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();

        let present = if state.groups.remove(group_id).is_some() {
            // Delete all existing group membership for this group id
            for stored_part in state.users.values_mut() {
                if let Some(groups) = &mut stored_part.resource.groups {
                    groups.retain(|user_group| {
                        user_group.value.as_deref() != Some(group_id)
                    });
                }
            }

            true
        } else {
            false
        };

        Ok(present)
    }
}
