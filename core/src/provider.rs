// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use slog::{Logger, error};

use super::*;

fn provider_error_to_error(
    log: &Logger,
    context: String,
) -> impl FnOnce(ProviderStoreError) -> Error {
    move |error| {
        match error {
            ProviderStoreError::StoreError(mut error) => {
                // Add context to the anyhow error and log it for our own records
                // and only pass along the context message to outside consumers of
                // the API.
                error = error.context(context.clone());
                // NB: Using the "#?" formatter is load bearing as it will
                // inline the entire error chain.
                error!(log, "{error:#?}");
                Error::internal_error(context)
            }
            // We don't log the raw scim error json
            ProviderStoreError::Scim(error) => error,
        }
    }
}

/// Provider implements SCIM CRUD over some provider store, transforming the
/// Rust types returned by that store into the generic SCIM response types.
pub struct Provider<T: ProviderStore> {
    log: Logger,
    store: T,
}

impl<T: ProviderStore> Provider<T> {
    pub fn new(log: Logger, store: T) -> Self {
        Self { log, store }
    }

    pub async fn list_users(
        &self,
        query_params: QueryParams,
    ) -> Result<ListResponse, Error> {
        let stored_users =
            self.store.list_users(query_params.filter()).await.map_err(
                provider_error_to_error(
                    &self.log,
                    "list users failed!".to_string(),
                ),
            )?;

        ListResponse::from_resources(stored_users, query_params)
    }

    pub async fn get_user_by_id(
        &self,
        query_params: QueryParams,
        user_id: &str,
    ) -> Result<SingleResourceResponse, Error> {
        let StoredParts { resource, meta } = self
            .store
            .get_user_by_id(user_id)
            .await
            .map_err(provider_error_to_error(
                &self.log,
                format!("get user by id {user_id} failed!"),
            ))?
            .ok_or(Error::not_found(user_id.to_string()))?;

        SingleResourceResponse::from_resource(
            resource,
            meta,
            Some(query_params),
        )
    }

    pub async fn create_user(
        &self,
        request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        // `groups` is readOnly, so clients cannot add users to groups when
        // creating new users.
        if let Some(groups) = &request.groups {
            if !groups.is_empty() {
                return Err(Error::mutability(
                    "attribute groups is readOnly".to_string(),
                ));
            }
        }

        let StoredParts { resource, meta } =
            self.store.create_user(request).await.map_err(
                provider_error_to_error(
                    &self.log,
                    "create user failed!".to_string(),
                ),
            )?;

        SingleResourceResponse::from_resource(resource, meta, None)
    }

    pub async fn replace_user(
        &self,
        user_id: &str,
        request: CreateUserRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let StoredParts { resource, meta } =
            self.store.replace_user(user_id, request).await.map_err(
                provider_error_to_error(
                    &self.log,
                    format!("replace user by id {user_id} failed!"),
                ),
            )?;

        SingleResourceResponse::from_resource(resource, meta, None)
    }

    pub async fn patch_user(
        &self,
        user_id: &str,
        request: PatchRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_user = self
            .store
            .get_user_by_id(user_id)
            .await
            .map_err(provider_error_to_error(
                &self.log,
                format!("patch user by id {user_id} failed!"),
            ))?
            .ok_or(Error::not_found(user_id.to_string()))?;

        let StoredParts { resource: user, meta: _ } =
            request.apply_user_ops(&stored_user)?;

        let request = CreateUserRequest {
            name: user.name,
            active: user.active,
            external_id: user.external_id,
            groups: user.groups,
        };

        let StoredParts { resource, meta } =
            self.store.replace_user(user_id, request).await.map_err(
                provider_error_to_error(
                    &self.log,
                    format!("replace user by id {user_id} failed!"),
                ),
            )?;

        SingleResourceResponse::from_resource(resource, meta, None)
    }

    pub async fn delete_user(
        &self,
        user_id: &str,
    ) -> Result<Response<Body>, Error> {
        match self.store.delete_user_by_id(user_id).await.map_err(
            provider_error_to_error(
                &self.log,
                format!("delete user by id {user_id} failed!"),
            ),
        )? {
            ProviderStoreDeleteResult::Deleted => deleted_http_response(),

            ProviderStoreDeleteResult::NotFound => {
                Err(Error::not_found(user_id.to_string()))
            }
        }
    }

    pub async fn list_groups(
        &self,
        query_params: QueryParams,
    ) -> Result<ListResponse, Error> {
        let stored_groups =
            self.store.list_groups(query_params.filter()).await.map_err(
                provider_error_to_error(
                    &self.log,
                    "list groups failed!".to_string(),
                ),
            )?;

        ListResponse::from_resources(stored_groups, query_params)
    }

    pub async fn get_group_by_id(
        &self,
        query_params: QueryParams,
        group_id: &str,
    ) -> Result<SingleResourceResponse, Error> {
        let StoredParts { resource: group, meta } = self
            .store
            .get_group_by_id(group_id)
            .await
            .map_err(provider_error_to_error(
                &self.log,
                format!("get group by id {group_id} failed!"),
            ))?
            .ok_or(Error::not_found(group_id.to_string()))?;

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
        let StoredParts { resource: group, meta } =
            self.store.create_group(request).await.map_err(
                provider_error_to_error(
                    &self.log,
                    "create group failed!".to_string(),
                ),
            )?;

        SingleResourceResponse::from_resource(group, meta, None)
    }

    pub async fn replace_group(
        &self,
        group_id: &str,
        request: CreateGroupRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let StoredParts { resource: group, meta } =
            self.store.replace_group(group_id, request).await.map_err(
                provider_error_to_error(
                    &self.log,
                    format!("replace group by id {group_id} failed!"),
                ),
            )?;

        SingleResourceResponse::from_resource(group, meta, None)
    }

    pub async fn delete_group(
        &self,
        group_id: &str,
    ) -> Result<Response<Body>, Error> {
        match self.store.delete_group_by_id(group_id).await.map_err(
            provider_error_to_error(
                &self.log,
                format!("delete group by id {group_id} failed!"),
            ),
        )? {
            ProviderStoreDeleteResult::Deleted => deleted_http_response(),

            ProviderStoreDeleteResult::NotFound => {
                Err(Error::not_found(group_id.to_string()))
            }
        }
    }

    pub async fn patch_group(
        &self,
        group_id: &str,
        request: PatchRequest,
    ) -> Result<SingleResourceResponse, Error> {
        let stored_group = self
            .store
            .get_group_by_id(group_id)
            .await
            .map_err(provider_error_to_error(
                &self.log,
                format!("patch group by id {group_id} failed!"),
            ))?
            .ok_or(Error::not_found(group_id.to_string()))?;

        let StoredParts { resource: group, meta: _ } =
            request.apply_group_ops(&stored_group)?;

        let request = CreateGroupRequest {
            display_name: group.display_name,
            external_id: group.external_id,
            members: group.members,
        };

        self.replace_group(group_id, request).await
    }
}

impl Provider<InMemoryProviderStore> {
    pub fn state(&self) -> InMemoryProviderStoreState {
        self.store.state()
    }
}
