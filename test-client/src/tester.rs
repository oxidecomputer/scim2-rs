// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Context;
use anyhow::bail;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::blocking::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use scim2_rs::Group;
use scim2_rs::ListResponse;
use scim2_rs::Resource;
use scim2_rs::SingleResourceResponse;
use scim2_rs::StoredMeta;
use scim2_rs::StoredParts;
use scim2_rs::User;

pub struct Tester {
    url: String,
    client: Client,
}

impl Tester {
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

        let sales_reps =
            self.create_empty_group().context("create_empty_group")?;

        self.replace_group_test(&sales_reps).context("replace_group_test")?;

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
        let random_id = "999999";

        // A GET of nonexistent user = 404
        let result = self
            .client
            .get(format!("{}/Users/{}", self.url, random_id))
            .send()?;

        assert_eq!(result.status(), StatusCode::NOT_FOUND);

        // DELETE of nonexistent user = 404
        let result = self
            .client
            .delete(format!("{}/Users/{}", self.url, random_id))
            .send()?;

        assert_eq!(result.status(), StatusCode::NOT_FOUND);

        // A GET of nonexistent group = 404
        let result = self
            .client
            .get(format!("{}/Groups/{}", self.url, random_id))
            .send()?;

        assert_eq!(result.status(), StatusCode::NOT_FOUND);

        // DELETE of nonexistent group = 404
        let result = self
            .client
            .delete(format!("{}/Groups/{}", self.url, random_id))
            .send()?;

        assert_eq!(result.status(), StatusCode::NOT_FOUND);

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

        if error.status()? != StatusCode::CONFLICT {
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

        if error.status()? != StatusCode::CONFLICT {
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

        // Leave out active and external id, and validate they don't change

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

            if check_user.active != jim.active {
                bail!(
                    "active changed from {:?} to {:?}",
                    jim.active,
                    check_user.active
                );
            }

            if check_user.external_id != jim.external_id {
                bail!(
                    "external_id changed from {:?} to {:?}",
                    jim.external_id,
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

        // It's possible to create a duplicate group: RFC 7644 says that SCIM2
        // groups do not have any fields (mainly external id, display name)
        // where uniqueness is required in the POST request.

        let conflict_result = self
            .client
            .post(format!("{}/Groups", self.url))
            .json(&body)
            .send()?;

        if conflict_result.status() != StatusCode::CREATED {
            bail!(
                "conflicting POST returned {} instead of {}",
                conflict_result.status(),
                StatusCode::CREATED
            );
        }

        // Compare group IDs, they should be different
        let conflicting_group: Group =
            self.result_as_resource(conflict_result)?.resource;

        if group.id() == conflicting_group.id() {
            bail!(
                "test group has same id ({}) as conflicting group ({})",
                group.id(),
                conflicting_group.id()
            );
        }

        // Delete the duplicate group

        let delete_result = self
            .client
            .delete(format!("{}/Groups/{}", self.url, conflicting_group.id()))
            .send()?;

        if delete_result.status() != StatusCode::NO_CONTENT {
            bail!(
                "DELETE returned {} instead of {}",
                delete_result.status(),
                StatusCode::NO_CONTENT
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
        // external ID - this should keep the external ID that was there
        // previously.

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

        if new_group != *group {
            bail!("group PUT didn't work, different group returned")
        }

        // External ID should be the same.

        let Some(new_group_external_id) = &new_group.external_id else {
            bail!(
                "group's external ID should be Some, its {:?}",
                new_group.external_id
            );
        };

        if Some(new_group_external_id) != group.external_id.as_ref() {
            bail!(
                "group's external ID changed from {:?} to {:?}",
                group.external_id,
                new_group.external_id,
            );
        }

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
}
