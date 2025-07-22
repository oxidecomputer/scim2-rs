// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[serde(rename = "userName")]
    pub name: String,

    pub active: Option<bool>,

    /// An identifier for the resource as defined by the provisioning client
    pub external_id: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,

    #[serde(rename = "userName")]
    pub name: String,

    pub active: bool,

    /// An identifier for the resource as defined by the provisioning client
    // This is an OPTIONAL attribute, so skip serializing it if it's null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

impl Resource for User {
    fn schema() -> String {
        String::from("urn:ietf:params:scim:schemas:core:2.0:User")
    }

    fn resource_type() -> String {
        String::from("User")
    }
}
