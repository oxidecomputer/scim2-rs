// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

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
        self.store
            .list_users(query_params.clone())
            .await
            .map_err(err_with_context("list users failed!".to_string()))
            .map(|users| ListResponse::from_resources(users, query_params))?
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

        let (user, meta) = stored_user.into();
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

        let (user, meta) = stored_user.into();
        SingleResourceResponse::from_resource(user, meta, None)
    }

    pub async fn replace_user(
        &self,
        _user_id: String,
        _request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn delete_user(
        &self,
        user_id: String,
    ) -> Result<Response<Body>, Error> {
        self.store
            .delete_user_by_id(user_id.clone())
            .await
            .map_err(err_with_context(format!(
                "delete user by id {user_id} failed!"
            )))
            .map(|_| deleted_http_response())?
    }

    pub async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<ListResponse, Error> {
        self.store
            .list_groups(query_params.clone())
            .await
            .map_err(err_with_context("list groups failed!".to_string()))
            .map(|groups| ListResponse::from_resources(groups, query_params))?
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

        let (group, meta) = stored_group.into();

        SingleResourceResponse::from_resource::<Group>(
            group,
            meta,
            Some(query_params),
        )
    }

    pub async fn create_group(
        &self,
        request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_group =
            self.store.create_group(request).await.map_err(
                err_with_context("create group failed!".to_string()),
            )?;

        let (group, meta) = stored_group.into();
        SingleResourceResponse::from_resource(group, meta, None)
    }

    pub async fn replace_group(
        &self,
        _group_id: String,
        _request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        unimplemented!()
    }

    pub async fn delete_group(
        &self,
        group_id: String,
    ) -> Result<Response<Body>, Error> {
        self.store
            .delete_group_by_id(group_id.clone())
            .await
            .map_err(err_with_context(format!(
                "delete group by id {group_id} failed!"
            )))
            .map(|_| deleted_http_response())?
    }
}
