// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

/// Provider implements SCIM CRUD over some provider store, transforming the
/// Rust types returned by that store into the generic SCIM response types.
pub struct Provider<T: ProviderStore> {
    store: T,
}

impl<T: ProviderStore> Provider<T> {
    pub fn new(store: T) -> Self {
        Self { store }
    }

    pub async fn list_users(&self, query_params: QueryParams) -> Result<ListResponse, Error> {
        let users: Vec<User> = match self.store.list_users(query_params.clone()).await {
            Ok(users) => users,
            Err(e) => {
                return match e {
                    ProviderStoreError::Scim(e) => Err(e),

                    ProviderStoreError::StoreError(e) => {
                        Err(Error::internal_error(format!("list users failed! {e}")))
                    }
                };
            }
        };

        ListResponse::from_resources(users, query_params)
    }

    pub async fn get_user_by_id(
        &self,
        query_params: QueryParams,
        user_id: String,
    ) -> Result<SingleResourceResponse, Error> {
        let user = match self.store.get_user_by_id(user_id.clone()).await {
            Ok(user) => user,

            Err(e) => {
                return match e {
                    ProviderStoreError::Scim(e) => Err(e),

                    ProviderStoreError::StoreError(e) => {
                        Err(Error::internal_error(format!("get user by id failed! {e}")))
                    }
                };
            }
        };

        let Some(user) = user else {
            return Err(Error::not_found(user_id));
        };

        SingleResourceResponse::from_resource(user, query_params)
    }

    pub async fn create_user(
        &self,
        _request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn replace_user(
        &self,
        _user_id: String,
        _request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn delete_user(&self, _user_id: String) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn list_groups(&self, query_params: QueryParams) -> Result<ListResponse, Error> {
        let groups: Vec<Group> = match self.store.list_groups(query_params.clone()).await {
            Ok(groups) => groups,
            Err(e) => {
                return match e {
                    ProviderStoreError::Scim(e) => Err(e),

                    ProviderStoreError::StoreError(e) => {
                        Err(Error::internal_error(format!("list groups failed! {e}")))
                    }
                };
            }
        };

        ListResponse::from_resources(groups, query_params)
    }

    pub async fn get_group_by_id(
        &self,
        query_params: QueryParams,
        group_id: String,
    ) -> Result<SingleResourceResponse, Error> {
        let group = match self.store.get_group_by_id(group_id.clone()).await {
            Ok(group) => group,

            Err(e) => {
                return match e {
                    ProviderStoreError::Scim(e) => Err(e),

                    ProviderStoreError::StoreError(e) => Err(Error::internal_error(format!(
                        "get group by id failed! {e}"
                    ))),
                };
            }
        };

        let Some(group) = group else {
            return Err(Error::not_found(group_id));
        };

        SingleResourceResponse::from_resource(group, query_params)
    }

    pub async fn create_group(
        &self,
        _request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn replace_group(
        &self,
        _group_id: String,
        _request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn delete_group(&self, _group_id: String) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }
}
