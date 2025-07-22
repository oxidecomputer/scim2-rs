// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

struct MappedError {
    inner: ProviderStoreError,
    context: &'static str,
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
    fn with_context(self, context: &'static str) -> MappedError {
        match self {
            inner @ ProviderStoreError::StoreError(_) => {
                MappedError { inner, context }
            }
            inner @ ProviderStoreError::Scim(_) => {
                MappedError { inner, context: "" }
            }
        }
    }
}

/// Create a `MappedError` with the provided context
fn err_with_context(
    context: &'static str,
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
    ) -> Result<ListResponse<User>, Error> {
        self.store
            .list_users(query_params.clone())
            .await
            .map_err(err_with_context("list users failed!"))
            .map(|users| ListResponse::from_resources(users, query_params))?
    }

    pub async fn get_user_by_id(
        &self,
        query_params: QueryParams,
        user_id: String,
    ) -> Result<SingleResourceResponse<User>, Error> {
        let user = self
            .store
            .get_user_by_id(user_id.clone())
            .await
            .map_err(err_with_context("get user by id failed!"))?;

        let Some(user) = user else {
            return Err(Error::not_found(user_id));
        };

        SingleResourceResponse::from_resource::<User>(user, Some(query_params))
    }

    pub async fn create_user(
        &self,
        request: CreateUserRequest,
    ) -> Result<SingleResourceResponse<User>, Error> {
        self.store
            .create_user(request)
            .await
            .map_err(err_with_context("create user failed!"))
            .map(|user| {
                SingleResourceResponse::from_resource::<User>(user, None)
            })?
    }

    pub async fn replace_user(
        &self,
        _user_id: String,
        _request: CreateUserRequest,
    ) -> Result<SingleResourceResponse<User>, Error> {
        unimplemented!()
    }

    pub async fn delete_user(
        &self,
        user_id: String,
    ) -> Result<Response<Body>, Error> {
        self.store
            .delete_user_by_id(user_id.clone())
            .await
            .map_err(err_with_context("delete user by id failed!"))
            .map(|_| deleted_http_response())?
    }

    pub async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<ListResponse<Group>, Error> {
        self.store
            .list_groups(query_params.clone())
            .await
            .map_err(err_with_context("list groups failed!"))
            .map(|groups| ListResponse::from_resources(groups, query_params))?
    }

    pub async fn get_group_by_id(
        &self,
        query_params: QueryParams,
        group_id: String,
    ) -> Result<SingleResourceResponse<Group>, Error> {
        let group = self
            .store
            .get_group_by_id(group_id.clone())
            .await
            .map_err(err_with_context("get group by id failed!"))?;

        let Some(group) = group else {
            return Err(Error::not_found(group_id));
        };

        SingleResourceResponse::from_resource::<Group>(
            group,
            Some(query_params),
        )
    }

    pub async fn create_group(
        &self,
        request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse<Group>, Error> {
        self.store
            .create_group(request)
            .await
            .map_err(err_with_context("create group failed!"))
            .map(|group| {
                SingleResourceResponse::from_resource::<Group>(group, None)
            })?
    }

    pub async fn replace_group(
        &self,
        _group_id: String,
        _request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse<Group>, Error> {
        unimplemented!()
    }

    pub async fn delete_group(
        &self,
        group_id: String,
    ) -> Result<Response<Body>, Error> {
        self.store
            .delete_group_by_id(group_id.clone())
            .await
            .map_err(err_with_context("delete group by id failed!"))
            .map(|_| deleted_http_response())?
    }
}
