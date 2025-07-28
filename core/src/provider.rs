// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

use std::str::FromStr;

struct MappedError {
    inner: ProviderStoreError,
    context: String,
}

impl From<MappedError> for Error {
    fn from(value: MappedError) -> Self {
        match value.inner {
            ProviderStoreError::StoreError(error) => {
                // XXX we want to use err_chain here probably...
                Error::internal_error(format!("{} {error}", value.context))
            }
            ProviderStoreError::Scim(error) => error,
        }
    }
}

impl ProviderStoreError {
    fn with_context(self, context: String) -> MappedError {
        match self {
            inner @ ProviderStoreError::StoreError(_) => {
                MappedError { inner, context }
            }
            inner @ ProviderStoreError::Scim(_) => {
                MappedError { inner, context: String::new() }
            }
        }
    }
}

/// Create a `MappedError` with the provided context
fn err_with_context(
    context: String,
) -> impl FnOnce(ProviderStoreError) -> MappedError {
    move |e| e.with_context(context)
}

/// Provider implements SCIM CRUD over some provider store, transforming the
/// Rust types returned by that store into the generic SCIM response types.
pub struct Provider<T: ProviderStore> {
    store: T,
}

impl<T: ProviderStore> Provider<T> {
    pub fn new(store: T) -> Self {
        Self { store }
    }

    pub async fn list_users(
        &self,
        query_params: QueryParams,
    ) -> Result<ListResponse, Error> {
        let stored_users = self
            .store
            .list_users(query_params.clone())
            .await
            .map_err(err_with_context("list users failed!".to_string()))?;

        let mut users: Vec<StoredParts<User>> =
            Vec::with_capacity(stored_users.len());

        for stored_user in stored_users {
            let members = self
                .store
                .get_user_group_membership(stored_user.id.clone())
                .await
                .map_err(err_with_context(
                    "get_user_group_membership failed!".to_string(),
                ))?;

            users.push(StoredParts::<User>::from(stored_user, members));
        }

        ListResponse::from_resources(users, query_params)
    }

    pub async fn get_user_by_id(
        &self,
        query_params: QueryParams,
        user_id: String,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_user =
            self.store.get_user_by_id(user_id.clone()).await.map_err(
                err_with_context(format!("get user by id {user_id} failed!")),
            )?;

        let Some(stored_user) = stored_user else {
            return Err(Error::not_found(user_id));
        };

        let members = self
            .store
            .get_user_group_membership(stored_user.id.clone())
            .await
            .map_err(err_with_context(
                "get_user_group_membership failed!".to_string(),
            ))?;

        let StoredParts { resource: user, meta } =
            StoredParts::<User>::from(stored_user, members);
        SingleResourceResponse::from_resource(user, meta, Some(query_params))
    }

    pub async fn create_user(
        &self,
        request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_user = self
            .store
            .create_user(request)
            .await
            .map_err(err_with_context("create user failed!".to_string()))?;

        // `groups` is readOnly, so clients cannot add users to groups when
        // creating new users.
        let StoredParts { resource: user, meta } =
            StoredParts::<User>::from(stored_user, vec![]);
        SingleResourceResponse::from_resource(user, meta, None)
    }

    pub async fn replace_user(
        &self,
        user_id: String,
        request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_user =
            self.store.replace_user(user_id.clone(), request).await.map_err(
                err_with_context(format!(
                    "replace user by id {user_id} failed!"
                )),
            )?;

        let members = self
            .store
            .get_user_group_membership(stored_user.id.clone())
            .await
            .map_err(err_with_context(
                "get_user_group_membership failed!".to_string(),
            ))?;

        let StoredParts { resource: user, meta } =
            StoredParts::<User>::from(stored_user, members);
        SingleResourceResponse::from_resource(user, meta, None)
    }

    pub async fn delete_user(
        &self,
        user_id: String,
    ) -> Result<Response<Body>, Error> {
        let maybe_stored_user =
            self.store.delete_user_by_id(user_id.clone()).await.map_err(
                err_with_context(format!(
                    "delete user by id {user_id} failed!"
                )),
            )?;

        match maybe_stored_user {
            Some(_) => deleted_http_response(),

            None => Err(Error::not_found(user_id)),
        }
    }

    pub async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<ListResponse, Error> {
        let stored_groups = self
            .store
            .list_groups(query_params.clone())
            .await
            .map_err(err_with_context("list groups failed!".to_string()))?;

        let mut groups: Vec<StoredParts<Group>> =
            Vec::with_capacity(stored_groups.len());

        for stored_group in stored_groups {
            groups.push(StoredParts::<Group>::from(stored_group));
        }

        ListResponse::from_resources(groups, query_params)
    }

    pub async fn get_group_by_id(
        &self,
        query_params: QueryParams,
        group_id: String,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_group =
            self.store.get_group_by_id(group_id.clone()).await.map_err(
                err_with_context(format!("get group by id {group_id} failed!")),
            )?;

        let Some(stored_group) = stored_group else {
            return Err(Error::not_found(group_id));
        };

        let StoredParts { resource: group, meta } =
            StoredParts::<Group>::from(stored_group);

        SingleResourceResponse::from_resource::<Group>(
            group,
            meta,
            Some(query_params),
        )
    }

    async fn get_group_members(
        &self,
        members: &[GroupMember],
    ) -> Result<Vec<StoredGroupMember>, Error> {
        let mut stored_members: Vec<StoredGroupMember> =
            Vec::with_capacity(members.len());

        for member in members {
            let GroupMember { resource_type, value } = member;

            let Some(value) = value else {
                // The minimum that this code needs is the value field so
                // complain about that.
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
                        let maybe_user = self
                            .store
                            .get_user_by_id(value.clone())
                            .await
                            .map_err(err_with_context(
                                "get_user_by_id".to_string(),
                            ))?;

                        if maybe_user.is_none() {
                            return Err(Error::not_found(value.clone()));
                        }
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
                let maybe_user =
                    self.store.get_user_by_id(value.clone()).await.map_err(
                        err_with_context("get_group_members".to_string()),
                    )?;

                let maybe_group =
                    self.store.get_group_by_id(value.clone()).await.map_err(
                        err_with_context("get_group_members".to_string()),
                    )?;

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

            let stored_member =
                StoredGroupMember { resource_type, value: value.clone() };

            stored_members.push(stored_member);
        }

        Ok(stored_members)
    }

    pub async fn create_group(
        &self,
        request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_members = if let Some(members) = &request.members {
            self.get_group_members(members).await?
        } else {
            vec![]
        };

        let stored_group = self
            .store
            .create_group_with_members(request, stored_members.clone())
            .await
            .map_err(err_with_context("create group failed!".to_string()))?;

        let StoredParts { resource: group, meta } =
            StoredParts::<Group>::from(stored_group);
        SingleResourceResponse::from_resource(group, meta, None)
    }

    pub async fn replace_group(
        &self,
        group_id: String,
        request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_members = if let Some(members) = &request.members {
            self.get_group_members(members).await?
        } else {
            vec![]
        };

        let stored_group = self
            .store
            .replace_group_with_members(
                group_id.clone(),
                request,
                stored_members.clone(),
            )
            .await
            .map_err(err_with_context(format!(
                "replace group by id {group_id} failed!"
            )))?;

        let StoredParts { resource: group, meta } =
            StoredParts::<Group>::from(stored_group);
        SingleResourceResponse::from_resource(group, meta, None)
    }

    pub async fn delete_group(
        &self,
        group_id: String,
    ) -> Result<Response<Body>, Error> {
        let maybe_stored_group =
            self.store.delete_group_by_id(group_id.clone()).await.map_err(
                err_with_context(format!(
                    "delete group by id {group_id} failed!"
                )),
            )?;

        match maybe_stored_group {
            Some(_) => deleted_http_response(),

            None => Err(Error::not_found(group_id)),
        }
    }
}

impl Provider<InMemoryProviderStore> {
    pub fn state(&self) -> InMemoryProviderStoreState {
        self.store.state()
    }
}
