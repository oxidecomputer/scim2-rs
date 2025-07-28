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

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,

    #[serde(rename = "userName")]
    pub name: String,

    pub active: Option<bool>,

    /// An identifier for the resource as defined by the provisioning client
    // This is an OPTIONAL attribute, so skip serializing it if it's null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    #[serde(skip_serializing_if = "skip_serializing_list::<UserGroup>")]
    pub groups: Option<Vec<UserGroup>>,
}

impl Resource for User {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn schema() -> String {
        String::from("urn:ietf:params:scim:schemas:core:2.0:User")
    }

    fn resource_type() -> String {
        String::from("User")
    }
}

/// A StoredUser is one that combines the fields in User and StoredMeta.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StoredUser {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub external_id: Option<String>,
    pub created: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub version: String,
}

impl StoredParts<User> {
    pub fn from(u: StoredUser, groups: Vec<UserGroup>) -> StoredParts<User> {
        let user = User {
            id: u.id,
            name: u.name,
            active: Some(u.active),
            external_id: u.external_id,
            groups: Some(groups),
        };

        let meta = StoredMeta {
            created: u.created,
            last_modified: u.last_modified,
            version: u.version,
        };

        StoredParts { resource: user, meta }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
pub enum UserGroupType {
    Direct,

    // Note: nested groups are not supported yet!
    Indirect,
}

impl std::str::FromStr for UserGroupType {
    type Err = String;

    fn from_str(r: &str) -> Result<Self, Self::Err> {
        match r.to_lowercase().as_str() {
            "direct" => Ok(UserGroupType::Direct),
            "indirect" => Ok(UserGroupType::Indirect),
            _ => Err(format!("{r} not a valid UserGroupType")),
        }
    }
}

impl std::fmt::Display for UserGroupType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UserGroupType::Direct => {
                write!(f, "direct")
            }

            UserGroupType::Indirect => {
                write!(f, "indirect")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct UserGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub member_type: Option<UserGroupType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}
