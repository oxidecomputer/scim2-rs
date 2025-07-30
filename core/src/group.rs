// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupRequest {
    pub display_name: String,

    /// An identifier for the resource as defined by the provisioning client
    pub external_id: Option<String>,

    pub members: Option<Vec<GroupMember>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,

    pub display_name: String,

    /// An identifier for the resource as defined by the provisioning client
    // This is an OPTIONAL attribute, so skip serializing it if it's null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    #[serde(skip_serializing_if = "skip_serializing_list::<GroupMember>")]
    pub members: Option<Vec<GroupMember>>,
}

impl Resource for Group {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn schema() -> String {
        String::from("urn:ietf:params:scim:schemas:core:2.0:Group")
    }

    fn resource_type() -> String {
        String::from("Group")
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
pub struct GroupMember {
    /// User or Group
    #[serde(rename = "type")]
    pub resource_type: Option<String>,

    /// identifier of the member of this group
    pub value: Option<String>,
}
