// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupRequest {
    pub display_name: String,

    /// An identifier for the resource as defined by the provisioning client
    pub external_id: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,

    pub display_name: String,

    /// An identifier for the resource as defined by the provisioning client
    // This is an OPTIONAL attribute, so skip serializing it if it's null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

impl Resource for Group {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn schema() -> String {
        ResourceType::Group.urn().to_string()
    }

    fn resource_type() -> ResourceType {
        ResourceType::Group
    }
}

/// A StoredGroup is one that combines the fields in Group and StoredMeta.
#[derive(Clone)]
pub struct StoredGroup {
    pub id: String,
    pub display_name: String,
    pub external_id: Option<String>,
    pub created: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub version: String,
}

impl From<StoredGroup> for StoredParts<Group> {
    fn from(u: StoredGroup) -> StoredParts<Group> {
        let group = Group {
            id: u.id,
            display_name: u.display_name,
            external_id: u.external_id,
        };

        let meta = StoredMeta {
            created: u.created,
            last_modified: u.last_modified,
            version: u.version,
        };

        StoredParts { resource: group, meta }
    }
}

#[derive(Debug, strum_macros::Display, strum_macros::EnumString, PartialEq)]
#[strum(ascii_case_insensitive)]
pub enum GroupAttr {
    // XXX ugh this is always required....
    #[strum(to_string = "id")]
    Id,
    #[strum(to_string = "displayName")]
    DisplayName,
    #[strum(to_string = "externalId")]
    ExternalId,
    Schemas,
    Meta,
}
