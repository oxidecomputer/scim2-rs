// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Deserialize, JsonSchema)]
pub struct CreateUserRequest {
    #[serde(rename = "userName")]
    pub name: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct User {
    pub id: String,

    #[serde(rename = "userName")]
    pub name: String,
}

impl Resource for User {
    fn schema() -> String {
        String::from("urn:ietf:params:scim:schemas:core:2.0:User")
    }

    fn resource_type() -> String {
        String::from("User")
    }
}
