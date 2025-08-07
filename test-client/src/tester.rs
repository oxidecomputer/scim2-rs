// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Context;
use anyhow::bail;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::blocking::Client;
use reqwest::header;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::str::FromStr;
use uuid::Uuid;

use scim2_rs::Group;
use scim2_rs::GroupMember;
use scim2_rs::ListResponse;
use scim2_rs::Resource;
use scim2_rs::ResourceType;
use scim2_rs::SingleResourceResponse;
use scim2_rs::StoredMeta;
use scim2_rs::StoredParts;
use scim2_rs::User;

pub struct Tester {
    url: String,
    client: Client,
}

impl Tester {
    pub fn new_with_bearer_auth(
        url: String,
        bearer: String,
    ) -> anyhow::Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {bearer}"))?,
        );

        let client = Client::builder().default_headers(headers).build()?;

        Ok(Self { url, client })
    }

    pub fn new(url: String) -> Self {
        Self { url, client: Client::new() }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        self.nonexistent_resource_tests()
            .context("nonexistent_resource_tests")?;

        let dwight = self.create_user_tests().context("create_user_tests")?;
        let jim = self.create_jim_user().context("create_jim_user")?;

        self.list_users_test(&dwight, &jim).context("list_users_test")?;
        self.list_users_with_filter_test(&jim)
            .context("list_users_with_filter_test")?;

        self.replace_user_test(&jim).context("replace_user_test")?;

        self.patch_user_test(&jim).context("patch_user_test")?;

        let sales_reps =
            self.create_empty_group().context("create_empty_group")?;

        self.replace_group_test(&sales_reps).context("replace_group_test")?;

        self.patch_group_test(&sales_reps, &jim, &dwight)
            .context("patch_group_test")?;

        self.test_groups(&dwight, &jim).context("test_groups")?;

        Ok(())
    }

    fn result_as_resource<R>(
        &self,
        result: reqwest::blocking::Response,
    ) -> anyhow::Result<StoredParts<R>>
    where
        R: Resource + DeserializeOwned + Serialize,
    {
        let response: SingleResourceResponse = result.json()?;

        if !response.resource.schemas.contains(&R::schema()) {
            bail!("response does not contain {} schema", R::resource_type());
        }

        let resource: R =
            serde_json::from_value(serde_json::to_value(&response.resource)?)?;

        // XXX needs fixing
        //assert_eq!(
        //    response.meta.location,
        //    format!("{}/{}s/{}", self.url, R::resource_type(), resource.id())
        //);

        Ok(StoredParts { resource, meta: response.meta.into() })
    }

    fn result_as_resource_list<R>(
        &self,
        result: reqwest::blocking::Response,
    ) -> anyhow::Result<Vec<R>>
    where
        R: Resource + DeserializeOwned + Serialize,
    {
        let response: ListResponse = result.json()?;

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

    fn nonexistent_resource_tests(&self) -> anyhow::Result<()> {
        let random_id = Uuid::new_v4().to_string();

        // A GET of non-existent user = 404
        let result = self
            .client
            .get(format!("{}/Users/{}", self.url, random_id))
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "GET of non-existent user returned {} not {}",
                result.status(),
                StatusCode::NOT_FOUND,
            );
        }

        // PUT of non-existent user = 404
        let body = json!({
            "userName": "notblank",
        });

        let result = self
            .client
            .put(format!("{}/Users/{}", self.url, random_id))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "PUT of non-existent user returned {} not {}",
                result.status(),
                StatusCode::NOT_FOUND,
            );
        }

        // DELETE of non-existent user = 404
        let result = self
            .client
            .delete(format!("{}/Users/{}", self.url, random_id))
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "DELETE of non-existent user returned {} not {}",
                result.status(),
                StatusCode::NOT_FOUND,
            );
        }

        // A GET of non-existent group = 404
        let result = self
            .client
            .get(format!("{}/Groups/{}", self.url, random_id))
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "GET of non-existent group returned {} not {}",
                result.status(),
                StatusCode::NOT_FOUND,
            );
        }

        // PUT of non-existent group = 404
        let body = json!({
            "displayName": "notblank",
        });

        let result = self
            .client
            .put(format!("{}/Groups/{}", self.url, random_id))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "PUT of non-existent group returned {} not {}",
                result.status(),
                StatusCode::NOT_FOUND,
            );
        }

        // DELETE of non-existent group = 404
        let result = self
            .client
            .delete(format!("{}/Groups/{}", self.url, random_id))
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "DELETE of non-existent group returned {} not {}",
                result.status(),
                StatusCode::NOT_FOUND,
            );
        }

        Ok(())
    }

    fn create_user_tests(&self) -> anyhow::Result<User> {
        let body = json!({
            "userName": "dschrute",
            "externalId": "dschrute@dundermifflin.com",
        });

        let result = self
            .client
            .post(format!("{}/Users", self.url))
            .json(&body)
            .send()?;

        // RFC 7664 § 3.3:
        // When the service provider successfully creates the new resource, an
        // HTTP response SHALL be returned with HTTP status code 201 (Created).
        if result.status() != StatusCode::CREATED {
            bail!("POST to /Users returned status code {}", result.status());
        }

        let user: User = self.result_as_resource(result)?.resource;

        if user.name != "dschrute" {
            bail!("user name of test user is {}, not dschrute", user.name);
        }

        let Some(external_id) = &user.external_id else {
            bail!("external id of test user is None");
        };

        if external_id != "dschrute@dundermifflin.com" {
            bail!(
                "external id of test user is {}, not dschrute@dundermifflin.com",
                external_id
            );
        }

        // RFC 7664 § 3.3:
        // If the service provider determines that the creation of the requested
        // resource conflicts with existing resources (e.g., a "User" resource
        // with a duplicate "userName"), the service provider MUST return HTTP
        // status code 409 (Conflict) with a "scimType" error code of
        // "uniqueness", as per Section 3.12.

        let conflict_result = self
            .client
            .post(format!("{}/Users", self.url))
            .json(&body)
            .send()?;

        if conflict_result.status() != StatusCode::CONFLICT {
            bail!(
                "conflicting POST returned {} instead of {}",
                conflict_result.status(),
                StatusCode::CONFLICT
            );
        }

        let error: scim2_rs::Error = conflict_result.json()?;

        if error.status() != StatusCode::CONFLICT {
            bail!(
                "SCIM error struct's status is {} instead of {}",
                error.status,
                StatusCode::CONFLICT
            );
        }

        let Some(error_type) = error.error_type else {
            bail!("SCIM error struct's error type is None");
        };

        if error_type != scim2_rs::ErrorType::Uniqueness {
            bail!(
                "SCIM error struct's error type is {:?} instead of {:?}",
                error_type,
                scim2_rs::ErrorType::Uniqueness
            );
        }

        Ok(user)
    }

    fn create_jim_user(&self) -> anyhow::Result<User> {
        let body = json!({
            "userName": "jhalpert",
            "externalId": "jhalpert@dundermifflin.com",
        });

        let result = self
            .client
            .post(format!("{}/Users", self.url))
            .json(&body)
            .send()?;

        let user: User = self.result_as_resource(result)?.resource;

        Ok(user)
    }

    fn list_users_test(&self, dwight: &User, jim: &User) -> anyhow::Result<()> {
        let result = self.client.get(format!("{}/Users", self.url)).send()?;

        if result.status() != StatusCode::OK {
            bail!("listing users returned {}", result.status());
        }

        let users: Vec<User> = self.result_as_resource_list(result)?;

        if users.len() != 2 {
            bail!("users length is {}, not 2", users.len());
        }

        if !users.contains(dwight) {
            bail!("users list does not contain dwight");
        }

        if !users.contains(jim) {
            bail!("users list does not contain jim");
        }

        Ok(())
    }

    fn list_users_with_filter_test(&self, jim: &User) -> anyhow::Result<()> {
        let mut url: Url = format!("{}/Users", self.url).parse().unwrap();
        url.set_query(Some(&format!("filter=username eq \"{}\"", jim.name)));

        let result = self.client.get(url).send()?;

        if result.status() != StatusCode::OK {
            bail!("listing users returned {}", result.status());
        }

        let users: Vec<User> = self.result_as_resource_list(result)?;

        if users.len() != 1 {
            bail!("users length is {}, not 1", users.len());
        }

        if !users.contains(jim) {
            bail!("users list does not contain jim");
        }

        Ok(())
    }

    fn patch_user_test(&self, jim: &User) -> anyhow::Result<()> {
        let url: Url =
            format!("{}/Users/{}", self.url, jim.id).parse().unwrap();

        // Set the users active field to false
        let body = json!(
            {
              "schemas": [
                "urn:ietf:params:scim:api:messages:2.0:PatchOp"
              ],
              "Operations": [
                {
                  "op": "replace",
                  "value": {
                    "active": false
                  }
                }
              ]
            }
        );

        let result = self.client.patch(url.clone()).json(&body).send()?;

        // RFC 7664 § 3.5.2:
        // On successful completion, the server either MUST return a 200 OK
        // response code and the entire resource within the response body,
        // subject to the "attributes" query parameter (see Section 3.9), or MAY
        // return HTTP status code 204 (No Content) and the appropriate response
        // headers for a successful PATCH request.  The server MUST return a 200
        // OK if the "attributes" parameter is specified in the request.
        if result.status() != StatusCode::OK {
            bail!(
                "PATCH returned {} instead of {}",
                result.status(),
                StatusCode::OK
            );
        }

        let jim: StoredParts<User> = self.result_as_resource(result)?;

        if jim.resource.active != Some(false) {
            bail!("users active field is not false",);
        }

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

        let result = self.client.patch(url).json(&body).send()?;
        let jim: StoredParts<User> = self.result_as_resource(result)?;

        if jim.resource.active != Some(true) {
            bail!("users active field is not true",);
        }

        // TODO Add some tests with invalid patch syntax

        Ok(())
    }

    fn replace_user_test(&self, jim: &User) -> anyhow::Result<()> {
        // Store Jim's meta for later comparison

        let jim_meta: StoredMeta = {
            let result = self
                .client
                .get(format!("{}/Users/{}", self.url, jim.id))
                .send()?;

            let parts: StoredParts<User> = self.result_as_resource(result)?;

            parts.meta
        };

        // Test replacing a user and changing the external id, aka: "You
        // seriously never noticed? Hats off to you!"

        let body = json!({
            "userName": "jhalpert",
            "externalId": "rpark@dundermifflin.com",
        });

        let result = self
            .client
            .put(format!("{}/Users/{}", self.url, jim.id))
            .json(&body)
            .send()?;

        // RFC 7664 § 3.5.1:
        // Unless otherwise specified, a successful PUT operation returns a 200
        // OK response code and the entire resource within the response body,
        // enabling the client to correlate the client's and the service
        // provider's views of the updated resource.

        if result.status() != StatusCode::OK {
            bail!(
                "PUT returned {} instead of {}",
                result.status(),
                StatusCode::OK
            );
        }

        let new_user: User = self.result_as_resource(result)?.resource;

        if new_user == *jim {
            bail!("user PUT didn't work, same user returned")
        }

        // The new user should be returned by the GET now.

        let result =
            self.client.get(format!("{}/Users/{}", self.url, jim.id)).send()?;

        let check: StoredParts<User> = self.result_as_resource(result)?;

        if new_user != check.resource {
            bail!("new user not returned after PUT");
        }

        // Assert the modification time changed

        if check.meta.last_modified == jim_meta.last_modified {
            bail!("last_modified didn't change after PUT");
        }

        // Revert the change.

        let result = self
            .client
            .put(format!("{}/Users/{}", self.url, jim.id))
            .json(&jim)
            .send()?;

        if result.status() != StatusCode::OK {
            bail!(
                "PUT returned {} instead of {}",
                result.status(),
                StatusCode::OK
            );
        }

        let old_user: User = self.result_as_resource(result)?.resource;

        if old_user != *jim {
            bail!("user revert PUT didn't work, new user returned")
        }

        // Test that replacing a user and using a duplicate username is not
        // allowed, aka: "Identity theft is no joke Jim!"

        let body = json!({
            "userName": "dschrute",
            "externalId": "jhalpert@dundermifflin.com",
        });

        let result = self
            .client
            .put(format!("{}/Users/{}", self.url, jim.id))
            .json(&body)
            .send()?;

        let error: scim2_rs::Error = result.json()?;

        if error.status() != StatusCode::CONFLICT {
            bail!(
                "SCIM error struct's status is {} instead of {}",
                error.status,
                StatusCode::CONFLICT
            );
        }

        let Some(error_type) = error.error_type else {
            bail!("SCIM error struct's error type is None");
        };

        if error_type != scim2_rs::ErrorType::Uniqueness {
            bail!(
                "SCIM error struct's error type is {:?} instead of {:?}",
                error_type,
                scim2_rs::ErrorType::Uniqueness
            );
        }

        // Leave out active and external id, and validate they do change

        let body = json!({
            "userName": "jhalpert",
        });

        let result = self
            .client
            .put(format!("{}/Users/{}", self.url, jim.id))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::OK {
            bail!(
                "PUT returned {} instead of {}",
                result.status(),
                StatusCode::OK
            );
        }

        {
            let check_user: User = self.result_as_resource(result)?.resource;

            if check_user.active.is_some() {
                bail!("active should be None, it's {:?}", check_user.active);
            }

            if check_user.external_id.is_some() {
                bail!(
                    "external_id should be None, it's {:?}",
                    check_user.external_id
                );
            }
        }

        Ok(())
    }

    fn create_empty_group(&self) -> anyhow::Result<Group> {
        let body = json!({
            "displayName": "Sales Reps",
            "externalId": "sales_reps",
            "members": [],
        });

        let result = self
            .client
            .post(format!("{}/Groups", self.url))
            .json(&body)
            .send()?;

        // RFC 7664 § 3.3:
        // When the service provider successfully creates the new resource, an
        // HTTP response SHALL be returned with HTTP status code 201 (Created).
        if result.status() != StatusCode::CREATED {
            bail!("POST to /Groups returned status code {}", result.status());
        }

        let group: Group = self.result_as_resource(result)?.resource;

        if group.display_name != "Sales Reps" {
            bail!(
                "user name of test group is {}, not Sales Reps",
                group.display_name
            );
        }

        let Some(external_id) = &group.external_id else {
            bail!("external id of test group is None");
        };

        if external_id != "sales_reps" {
            bail!(
                "external id of test group is {}, not sales_reps",
                external_id
            );
        }

        // It's not possible to create a duplicate group: RFC 7644 says that
        // SCIM2 groups do not have any fields (mainly external id, display
        // name) where uniqueness is required in the POST request, but the
        // in-memory provider store does not allow for duplicate groups.

        let conflict_result = self
            .client
            .post(format!("{}/Groups", self.url))
            .json(&body)
            .send()?;

        if conflict_result.status() != StatusCode::CONFLICT {
            bail!(
                "conflicting POST returned {} instead of {}",
                conflict_result.status(),
                StatusCode::CONFLICT
            );
        }

        Ok(group)
    }

    fn replace_group_test(&self, group: &Group) -> anyhow::Result<()> {
        // Store the group's meta for later comparison

        let group_meta: StoredMeta = {
            let result = self
                .client
                .get(format!("{}/Groups/{}", self.url, group.id))
                .send()?;

            let parts: StoredParts<Group> = self.result_as_resource(result)?;

            parts.meta
        };

        // Replace the existing group with the same name but without the
        // external ID - this nuke the external ID that was there previously.

        let body = json!({
            "displayName": "Sales Reps",
        });

        let result = self
            .client
            .put(format!("{}/Groups/{}", self.url, group.id))
            .json(&body)
            .send()?;

        // RFC 7664 § 3.5.1:
        // Unless otherwise specified, a successful PUT operation returns a 200
        // OK response code and the entire resource within the response body,
        // enabling the client to correlate the client's and the service
        // provider's views of the updated resource.

        if result.status() != StatusCode::OK {
            bail!(
                "PUT returned {} instead of {}",
                result.status(),
                StatusCode::OK
            );
        }

        let new_group: Group = self.result_as_resource(result)?.resource;

        // External ID should None

        if new_group.external_id.is_some() {
            bail!(
                "group's external ID should be Some, its {:?}",
                new_group.external_id
            );
        };

        // The new group should be returned by the GET now.

        let result = self
            .client
            .get(format!("{}/Groups/{}", self.url, group.id))
            .send()?;

        let check: StoredParts<Group> = self.result_as_resource(result)?;

        if new_group != check.resource {
            bail!("new group not returned after PUT");
        }

        // Assert the modification time changed

        if check.meta.last_modified == group_meta.last_modified {
            bail!("last_modified didn't change after PUT");
        }

        // Revert the change.

        let result = self
            .client
            .put(format!("{}/Groups/{}", self.url, group.id))
            .json(&group)
            .send()?;

        if result.status() != StatusCode::OK {
            bail!(
                "PUT returned {} instead of {}",
                result.status(),
                StatusCode::OK
            );
        }

        let old_group: Group = self.result_as_resource(result)?.resource;

        if old_group != *group {
            bail!("group revert PUT didn't work, new group returned")
        }

        Ok(())
    }

    fn patch_group_test(
        &self,
        group: &Group,
        jim: &User,
        dwight: &User,
    ) -> anyhow::Result<()> {
        let new_display_name = "Radiants";
        // Use a patch request to modify the groups displayName.
        let body = json!({
          "schemas": [
            "urn:ietf:params:scim:api:messages:2.0:PatchOp"
          ],
          "Operations": [
            {
              "op": "replace",
              "value": {
                "id": group.id,
                "displayName": new_display_name,
              }
            }
          ]
        });

        let result = self
            .client
            .patch(format!("{}/Groups/{}", self.url, group.id))
            .json(&body)
            .send()?;
        let patched_group: StoredParts<Group> =
            self.result_as_resource(result)?;

        // Make sure the group displayName has changed

        if patched_group.resource.display_name != new_display_name {
            bail!(
                "group displayName is {} but should be {new_display_name}",
                patched_group.resource.display_name
            )
        }

        // Set the displayName back to what it was
        let body = json!({
          "schemas": [
            "urn:ietf:params:scim:api:messages:2.0:PatchOp"
          ],
          "Operations": [
            {
              "op": "replace",
              "value": {
                "id": group.id,
                "displayName": group.display_name,
              }
            }
          ]
        });

        let result = self
            .client
            .patch(format!("{}/Groups/{}", self.url, group.id))
            .json(&body)
            .send()?;
        let patched_group: StoredParts<Group> =
            self.result_as_resource(result)?;

        // Make sure the group displayName was reverted

        if patched_group.resource.display_name != group.display_name {
            bail!(
                "group displayName is {} but should be {new_display_name}",
                patched_group.resource.display_name
            )
        }

        // Grab a handle to the stored group

        let result = self
            .client
            .get(format!("{}/Groups/{}", self.url, group.id))
            .send()?;

        let stored_group: StoredParts<Group> =
            self.result_as_resource(result)?;

        // Make sure we are starting with an empty group member list

        if !stored_group
            .resource
            .members
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            bail!(
                "group members should be empty but found: {:?}",
                stored_group.resource.members
            )
        }

        // Add the Jim and Dwight users in single PATCH request

        let body = json!({
          "schemas": [
            "urn:ietf:params:scim:api:messages:2.0:PatchOp"
          ],
          "Operations": [
            {
              "op": "add",
              "path": "members",
              "value": [
                {
                  "value": jim.id,
                  "display": jim.name
                },
                {
                  "value": dwight.id,
                  "display": dwight.name
                }
              ]
            }
          ]
        });

        let result = self
            .client
            .patch(format!("{}/Groups/{}", self.url, group.id))
            .json(&body)
            .send()?;
        let patched_group: StoredParts<Group> =
            self.result_as_resource(result)?;

        if !patched_group
            .resource
            .members
            .as_deref()
            .unwrap_or_default()
            .contains(&GroupMember {
                resource_type: Some(ResourceType::User.to_string()),
                value: Some(jim.id.clone()),
            })
        {
            bail!(
                "group members should contain {} but found {:?}",
                jim.id,
                patched_group.resource.members
            );
        }

        if !patched_group
            .resource
            .members
            .as_deref()
            .unwrap_or_default()
            .contains(&GroupMember {
                resource_type: Some(ResourceType::User.to_string()),
                value: Some(dwight.id.clone()),
            })
        {
            bail!(
                "group members should contain {} but found {:?}",
                dwight.id,
                patched_group.resource.members
            );
        }

        // Remove just the Jim user
        //
        let body = json!({
          "schemas": [
            "urn:ietf:params:scim:api:messages:2.0:PatchOp"
          ],
          "Operations": [
            {
              "op": "remove",
              "path": format!("members[value eq \"{}\"]", jim.id)
            }
          ]
        });

        let result = self
            .client
            .patch(format!("{}/Groups/{}", self.url, group.id))
            .json(&body)
            .send()?;

        let patched_group: StoredParts<Group> =
            self.result_as_resource(result)?;

        if patched_group
            .resource
            .members
            .as_deref()
            .unwrap_or_default()
            .contains(&GroupMember {
                resource_type: Some(ResourceType::User.to_string()),
                value: Some(jim.id.clone()),
            })
        {
            bail!(
                "group members should not contain {} but found {:?}",
                jim.id,
                patched_group.resource.members
            );
        }

        if patched_group.resource.members.as_deref().unwrap_or_default().len()
            != 1
        {
            bail!(
                "group members should only contain 1 member but found {}",
                patched_group
                    .resource
                    .members
                    .as_deref()
                    .unwrap_or_default()
                    .len()
            );
        }

        // Clear all group members

        let body = json!({
          "schemas": [
            "urn:ietf:params:scim:api:messages:2.0:PatchOp"
          ],
          "Operations": [
            {
              "op": "remove",
              "path": "members"
            }
          ]
        });

        let result = self
            .client
            .patch(format!("{}/Groups/{}", self.url, group.id))
            .json(&body)
            .send()?;

        let patched_group: StoredParts<Group> =
            self.result_as_resource(result)?;

        if !patched_group
            .resource
            .members
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            bail!(
                "group members should be empty but found: {:?}",
                stored_group.resource.members
            )
        }

        // TODO write a test when scim2-rs#24 is addressed that attempts to add
        // the same user to a group multiple times

        Ok(())
    }

    fn test_groups(&self, dwight: &User, jim: &User) -> anyhow::Result<()> {
        // The existing "Sales Reps" group should be empty

        let sales_reps_group_id = {
            let result =
                self.client.get(format!("{}/Groups", self.url)).send()?;

            if result.status() != StatusCode::OK {
                bail!("listing groups returned {}", result.status());
            }

            let mut groups: Vec<Group> =
                self.result_as_resource_list(result)?;

            if groups.len() != 1 {
                bail!("more than one group returned!");
            }

            if groups[0].display_name != "Sales Reps" {
                bail!("unexpected group returned!");
            }

            if let Some(members) = &groups[0].members {
                if !members.is_empty() {
                    bail!("existing Sales Reps group not empty!");
                }
            }

            groups.pop().unwrap().id
        };

        // And the existing user's groups should be empty

        {
            let result =
                self.client.get(format!("{}/Users", self.url)).send()?;

            if result.status() != StatusCode::OK {
                bail!("listing users returned {}", result.status());
            }

            let users: Vec<User> = self.result_as_resource_list(result)?;

            for user in &users {
                if let Some(groups) = &user.groups {
                    if !groups.is_empty() {
                        bail!("existing user {} groups not empty!", user.id);
                    }
                }
            }
        }

        // Adding a non-existent user to a group should be a 404

        let body = json!(
            {
              "schemas": [Group::schema()],
              "displayName": "Sales Reps",
              "members": [
                {
                  "value": Uuid::new_v4().to_string(),
                }
              ]
            }
        );

        let result = self
            .client
            .put(format!("{}/Groups/{}", self.url, sales_reps_group_id))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::NOT_FOUND {
            bail!(
                "PUT with non-existent user should be 404, not {}",
                result.status()
            );
        }

        // Add the first user to the group with PUT

        let body = json!(
            {
              "schemas": [Group::schema()],
              "displayName": "Sales Reps",
              "members": [
                {
                  "value": jim.id,
                }
              ]
            }
        );

        let result = self
            .client
            .put(format!("{}/Groups/{}", self.url, sales_reps_group_id))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::OK {
            bail!(
                "PUT with first user should be {}, not {}",
                StatusCode::OK,
                result.status(),
            );
        }

        // Now, that user should have this group in its `groups` field.

        {
            let result = self
                .client
                .get(format!("{}/Users/{}", self.url, jim.id))
                .send()?;

            if result.status() != StatusCode::OK {
                bail!(
                    "GET for first user should be {}, not {}",
                    StatusCode::OK,
                    result.status(),
                );
            }

            let first_user: User = self.result_as_resource(result)?.resource;

            let Some(first_user_groups) = &first_user.groups else {
                bail!("first user's groups should be Some");
            };

            if first_user_groups.len() != 1 {
                bail!(
                    "first user's group's len should be 1, not {}",
                    first_user_groups.len()
                );
            }

            if !first_user_groups.iter().any(|first_user_group| {
                first_user_group.value.as_ref() == Some(&sales_reps_group_id)
            }) {
                bail!(
                    "first user's group's value should contain {}",
                    sales_reps_group_id,
                );
            }
        }

        // And the group should have the user in its `members` field.

        {
            let result = self
                .client
                .get(format!("{}/Groups/{}", self.url, sales_reps_group_id))
                .send()?;

            if result.status() != StatusCode::OK {
                bail!(
                    "GET for group should be {}, not {}",
                    StatusCode::OK,
                    result.status(),
                );
            }

            let check: Group = self.result_as_resource(result)?.resource;

            let Some(check_members) = &check.members else {
                bail!("expected group to have Some members");
            };

            if check_members.len() != 1 {
                bail!(
                    "expected group's members to have len 1 not {}",
                    check_members.len()
                );
            }

            if !check_members
                .iter()
                .any(|member| member.value.as_ref() == Some(&jim.id))
            {
                bail!(
                    "user {} not present in group member list after first PUT",
                    jim.id
                );
            }

            if let Some(check_member_resource_type_string) =
                &check_members[0].resource_type
            {
                let check_member_resource_type = match ResourceType::from_str(
                    check_member_resource_type_string,
                ) {
                    Ok(value) => value,
                    Err(e) => bail!("ResourceType::from_str failed: {e}"),
                };

                if check_member_resource_type != ResourceType::User {
                    bail!("expected group's member to have resource type User");
                }
            }
        }

        // now, add the second user to the group

        let body = json!(
            {
              "schemas": [Group::schema()],
              "displayName": "Sales Reps",
              "members": [
                {
                  "value": jim.id,
                },
                {
                  "value": dwight.id,
                  "type": "User",
                }
              ]
            }
        );

        let result = self
            .client
            .put(format!("{}/Groups/{}", self.url, sales_reps_group_id))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::OK {
            bail!(
                "PUT with both users should be {}, not {}",
                StatusCode::OK,
                result.status(),
            );
        }

        // check that the group now has both users in its members field

        {
            let result = self
                .client
                .get(format!("{}/Groups/{}", self.url, sales_reps_group_id))
                .send()?;

            if result.status() != StatusCode::OK {
                bail!(
                    "GET for group should be {}, not {}",
                    StatusCode::OK,
                    result.status(),
                );
            }

            let check: Group = self.result_as_resource(result)?.resource;

            let Some(check_members) = &check.members else {
                bail!("expected group to have Some members");
            };

            if check_members.len() != 2 {
                bail!(
                    "expected group's members to have len 2 not {}",
                    check_members.len()
                );
            }

            if !check_members
                .iter()
                .any(|member| member.value.as_ref() == Some(&jim.id))
            {
                bail!(
                    "user {} not present in group member list after second PUT",
                    jim.id
                );
            }

            if !check_members
                .iter()
                .any(|member| member.value.as_ref() == Some(&dwight.id))
            {
                bail!(
                    "user {} not present in group member list after second PUT",
                    dwight.id
                );
            }
        }

        // add another group, importantly one that only Dwight is in

        let body = json!(
            {
              "schemas": [Group::schema()],
              "displayName": "Assistant to the Assistant to the Regional Manager",
              "members": [
                {
                  "value": dwight.id,
                }
              ]
            }
        );

        let result = self
            .client
            .post(format!("{}/Groups", self.url))
            .json(&body)
            .send()?;

        if result.status() != StatusCode::CREATED {
            bail!(
                "POST for aarm group should be {}, not {}",
                StatusCode::CREATED,
                result.status(),
            );
        }

        let aarm_group: Group = self.result_as_resource(result)?.resource;

        // This new group should only have Dwight in it

        {
            let Some(aarm_members) = &aarm_group.members else {
                bail!("expected AARM group to have Some members");
            };

            if aarm_members.len() != 1 {
                bail!(
                    "expected AARM group's members to have len 1 not {}",
                    aarm_members.len()
                );
            }

            if !aarm_members
                .iter()
                .any(|member| member.value.as_ref() == Some(&dwight.id))
            {
                bail!(
                    "user {} not present in group member list after second PUT",
                    dwight.id
                );
            }
        }

        // Dwight should have two groups in the User resource field

        {
            let result = self
                .client
                .get(format!("{}/Users/{}", self.url, dwight.id))
                .send()?;

            if result.status() != StatusCode::OK {
                bail!(
                    "GET for user should be {}, not {}",
                    StatusCode::OK,
                    result.status(),
                );
            }

            let check_dwight: User = self.result_as_resource(result)?.resource;

            let Some(check_dwight_groups) = &check_dwight.groups else {
                bail!("dwight's groups should be Some");
            };

            if check_dwight_groups.len() != 2 {
                bail!(
                    "dwight's group's len should be 2, not {}",
                    check_dwight_groups.len()
                );
            }

            if !check_dwight_groups.iter().any(|user_group| {
                user_group.value.as_ref() == Some(&aarm_group.id)
            }) {
                bail!(
                    "dwight user's group's value should contain {}",
                    aarm_group.id,
                );
            }

            if !check_dwight_groups.iter().any(|user_group| {
                user_group.value.as_ref() == Some(&sales_reps_group_id)
            }) {
                bail!(
                    "dwight user's group's value should contain {}",
                    sales_reps_group_id,
                );
            }
        }

        // Jim should have one still

        {
            let result = self
                .client
                .get(format!("{}/Users/{}", self.url, jim.id))
                .send()?;

            if result.status() != StatusCode::OK {
                bail!(
                    "GET for user should be {}, not {}",
                    StatusCode::OK,
                    result.status(),
                );
            }

            let check_jim: User = self.result_as_resource(result)?.resource;

            let Some(check_jim_groups) = &check_jim.groups else {
                bail!("jim's groups should be Some");
            };

            if check_jim_groups.len() != 1 {
                bail!(
                    "jim's group's len should be 1, not {}",
                    check_jim_groups.len()
                );
            }

            if !check_jim_groups.iter().any(|user_group| {
                user_group.value.as_ref() == Some(&sales_reps_group_id)
            }) {
                bail!(
                    "jim user's group's value should contain {}",
                    sales_reps_group_id,
                );
            }
        }

        // if the AARM group is deleted, dwight's membership should change

        {
            let result = self
                .client
                .delete(format!("{}/Groups/{}", self.url, aarm_group.id))
                .send()?;

            if result.status() != StatusCode::NO_CONTENT {
                bail!(
                    "DELETE for group should be {}, not {}",
                    StatusCode::NO_CONTENT,
                    result.status(),
                );
            }
        }

        {
            let result = self
                .client
                .get(format!("{}/Users/{}", self.url, dwight.id))
                .send()?;

            if result.status() != StatusCode::OK {
                bail!(
                    "GET for user should be {}, not {}",
                    StatusCode::OK,
                    result.status(),
                );
            }

            let check_dwight: User = self.result_as_resource(result)?.resource;

            let Some(check_dwight_groups) = &check_dwight.groups else {
                bail!("dwight's groups should be Some");
            };

            if check_dwight_groups.len() != 1 {
                bail!(
                    "jwm dwight's group's len should be 1, not {}",
                    check_dwight_groups.len()
                );
            }

            if !check_dwight_groups.iter().any(|user_group| {
                user_group.value.as_ref() == Some(&sales_reps_group_id)
            }) {
                bail!(
                    "dwight user's group's value should contain {}",
                    sales_reps_group_id,
                );
            }
        }

        Ok(())
    }
}
