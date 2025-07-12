// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Deserialize, JsonSchema)]
pub struct CreateGroupRequest {}

#[derive(Deserialize, JsonSchema)]
pub struct Group {}

impl Resource for Group {
    fn schema() -> String {
        String::from("urn:ietf:params:scim:schemas:core:2.0:Group")
    }

    fn resource_type() -> String {
        String::from("Group")
    }
}
