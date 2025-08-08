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
    ) -> Result<ProviderStoreDeleteResult, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();
        let maybe_user = state.users.remove(user_id);

        let result = if maybe_user.is_some() {
            ProviderStoreDeleteResult::Deleted
        } else {
            ProviderStoreDeleteResult::NotFound
        };

        Ok(result)
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
    ) -> Result<ProviderStoreDeleteResult, ProviderStoreError> {
        let mut state = self.state.lock().unwrap();

        let result = if state.groups.remove(group_id).is_some() {
            // Delete all existing group membership for this group id
            for stored_part in state.users.values_mut() {
                if let Some(groups) = &mut stored_part.resource.groups {
                    groups.retain(|user_group| {
                        user_group.value.as_deref() != Some(group_id)
                    });
                }
            }

            ProviderStoreDeleteResult::Deleted
        } else {
            ProviderStoreDeleteResult::NotFound
        };

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use anyhow::bail;
    use http::StatusCode;
    use reqwest::{Response, Url};
    use serde::{Serialize, de::DeserializeOwned};
    use serde_json::json;

    use crate::{
        ListResponse, Resource, SingleResourceResponse, StoredMeta,
        StoredParts, User,
    };

    struct ServerCtx {
        base_url: Url,
        client: reqwest::Client,
        _server_handle: tokio::task::JoinHandle<Result<(), String>>,
    }

    async fn setup() -> anyhow::Result<ServerCtx> {
        let server =
            scim2_test_provider_server::create_http_server(None).unwrap();
        let addr = server.local_addr();
        let base_url = format!("http://{addr}/v2").parse().unwrap();
        let server_handle = tokio::spawn(server);
        let client = reqwest::Client::new();

        Ok(ServerCtx { base_url, client, _server_handle: server_handle })
    }

    async fn result_as_resource<R>(
        result: reqwest::Response,
    ) -> anyhow::Result<StoredParts<R>>
    where
        R: Resource + DeserializeOwned + Serialize,
    {
        let response: SingleResourceResponse = result.json().await?;

        if !response.resource.schemas.contains(&R::schema()) {
            bail!("response does not contain {} schema", R::resource_type());
        }

        let resource: R =
            serde_json::from_value(serde_json::to_value(&response.resource)?)?;

        Ok(StoredParts { resource, meta: response.meta.into() })
    }

    async fn result_as_resource_list<R>(
        result: reqwest::Response,
    ) -> anyhow::Result<Vec<R>>
    where
        R: Resource + DeserializeOwned + Serialize,
    {
        let response: ListResponse = result.json().await?;

        let resources: Vec<R> =
            serde_json::from_value(serde_json::to_value(&response.resources)?)?;

        if response.total_results != response.resources.len() {
            // TODO: totalResults may be larger than the returned resources when
            // returning a single page of results
            bail!(
                "total results {} does not match resources list length {}",
                response.total_results,
                response.resources.len()
            );
        }

        Ok(resources)
    }

    async fn create_user(
        ctx: &ServerCtx,
        user_name: &str,
        external_id: &str,
    ) -> anyhow::Result<Response> {
        let body = json!({
            "userName": user_name,
            "externalId": external_id,
        });

        Ok(ctx
            .client
            .post(format!("{}/Users", ctx.base_url))
            .json(&body)
            .send()
            .await?)
    }

    async fn create_jim_user(
        ctx: &ServerCtx,
    ) -> anyhow::Result<(User, StoredMeta)> {
        let user_name = "jhalpert";
        let external_id = "jhalpert@dundermifflin.com";
        let result = create_user(ctx, user_name, external_id).await?;

        // User is created
        assert_eq!(result.status(), StatusCode::CREATED);

        let StoredParts::<User> { resource: user, meta } =
            result_as_resource(result).await?;
        assert_eq!(user.name, user_name);
        assert_eq!(user.external_id, Some(external_id.to_string()));

        Ok((user, meta))
    }

    async fn create_dwight_user(
        ctx: &ServerCtx,
    ) -> anyhow::Result<(User, StoredMeta)> {
        let user_name = "dschrute";
        let external_id = "dschrute@dundermifflin.com";
        let result = create_user(ctx, user_name, external_id).await?;

        // User is created
        assert_eq!(result.status(), StatusCode::CREATED);

        let StoredParts::<User> { resource: user, meta } =
            result_as_resource(result).await?;
        assert_eq!(user.name, user_name);
        assert_eq!(user.external_id, Some(external_id.to_string()));

        Ok((user, meta))
    }

    #[tokio::test]
    async fn test_create_user() {
        let ctx = setup().await.unwrap();
        let (jim, _meta) = create_jim_user(&ctx).await.unwrap();

        let conflict_result = create_user(
            &ctx,
            &jim.name,
            &jim.external_id.expect("jim has an externalId"),
        )
        .await
        .unwrap();

        // Creating the same user again should result in a conflict
        assert_eq!(conflict_result.status(), StatusCode::CONFLICT);
        let error: crate::Error = conflict_result.json().await.unwrap();
        assert_eq!(error.error_type.unwrap(), crate::ErrorType::Uniqueness)
    }

    #[tokio::test]
    async fn test_list_users() {
        let ctx = setup().await.unwrap();
        let (jim, _meta) = create_jim_user(&ctx).await.unwrap();
        let (dwight, _meta) = create_dwight_user(&ctx).await.unwrap();
        let result = ctx
            .client
            .get(format!("{}/Users", ctx.base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(result.status(), StatusCode::OK);
        let users: Vec<User> = result_as_resource_list(result).await.unwrap();

        assert!(users.contains(&jim));
        assert!(users.contains(&dwight));

        // Now test filtering for a specific user

        let mut url: Url = format!("{}/Users", ctx.base_url).parse().unwrap();
        url.set_query(Some(&format!("filter=username eq \"{}\"", jim.name)));

        let filtered_result = ctx.client.get(url).send().await.unwrap();
        assert_eq!(filtered_result.status(), StatusCode::OK);
        let filtered_users: Vec<User> =
            result_as_resource_list(filtered_result).await.unwrap();

        assert_eq!(filtered_users.len(), 1);
        assert!(filtered_users.contains(&jim));
    }

    #[tokio::test]
    async fn test_replace_user() {
        let ctx = setup().await.unwrap();
        let (jim, jim_meta) = create_jim_user(&ctx).await.unwrap();
        let _dwight_parts = create_dwight_user(&ctx).await.unwrap();

        // Test replacing a user and changing the external id, aka: "You
        // seriously never noticed? Hats off to you!"

        let body = json!({
            "userName": "jhalpert",
            "externalId": "rpark@dundermifflin.com",
        });
        let result = ctx
            .client
            .put(format!("{}/Users/{}", ctx.base_url, jim.id))
            .json(&body)
            .send()
            .await
            .unwrap();

        assert_eq!(result.status(), StatusCode::OK);
        let replaced_user: User =
            result_as_resource(result).await.unwrap().resource;

        assert_ne!(&jim, &replaced_user);

        // The new user should be returned by the GET now.

        let result = ctx
            .client
            .get(format!("{}/Users/{}", ctx.base_url, jim.id))
            .send()
            .await
            .unwrap();

        let check: StoredParts<User> =
            result_as_resource(result).await.unwrap();
        assert_eq!(&replaced_user, &check.resource);

        // Assert the modification time changed

        assert_ne!(check.meta.last_modified, jim_meta.last_modified);

        // Test that replacing a user and using a duplicate username is not
        // allowed, aka: "Identity theft is no joke Jim!"

        let body = json!({
            "userName": "dschrute",
            "externalId": replaced_user.external_id,
        });

        let result = ctx
            .client
            .put(format!("{}/Users/{}", ctx.base_url, jim.id))
            .json(&body)
            .send()
            .await
            .unwrap();

        assert_eq!(result.status(), StatusCode::CONFLICT);
        let error: crate::Error = result.json().await.unwrap();
        assert_eq!(error.error_type.unwrap(), crate::ErrorType::Uniqueness);

        // Leave out active and external id, and validate they do change

        let body = json!({
            "userName": "jhalpert",
        });

        let result = ctx
            .client
            .put(format!("{}/Users/{}", ctx.base_url, jim.id))
            .json(&body)
            .send()
            .await
            .unwrap();

        assert_eq!(result.status(), StatusCode::OK);
        let check_user: User =
            result_as_resource(result).await.unwrap().resource;
        assert!(check_user.active.is_none());
        assert!(check_user.external_id.is_none());
    }

    #[tokio::test]
    async fn test_patch_user() {
        let ctx = setup().await.unwrap();
        let (jim, _) = create_jim_user(&ctx).await.unwrap();

        // Verify we are starting with no value

        assert!(jim.active.is_none());

        // Set the users active field to true
        let body = json!(
            {
              "schemas": [
                "urn:ietf:params:scim:api:messages:2.0:PatchOp"
              ],
              "Operations": [
                {
                  "op": "replace",
                  "value": {
                    "active": true
                  }
                }
              ]
            }
        );

        let result = ctx
            .client
            .patch(format!("{}/Users/{}", ctx.base_url, &jim.id))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::OK);
        let user: User = result_as_resource(result).await.unwrap().resource;
        assert_eq!(user.active, Some(true));

        // Get the stored user and make sure it matches the returned user

        let result = ctx
            .client
            .get(format!("{}/Users/{}", ctx.base_url, &jim.id))
            .json(&body)
            .send()
            .await
            .unwrap();
        let patched_jim: User =
            result_as_resource(result).await.unwrap().resource;
        assert_eq!(patched_jim, user);
    }
}
