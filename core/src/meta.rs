// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Serialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub resource_type: String,

    pub created: DateTime<Utc>,

    pub last_modified: DateTime<Utc>,

    pub version: String,

    pub location: String,
}

#[derive(Clone)]
pub struct StoredMeta {
    pub created: DateTime<Utc>,

    pub last_modified: DateTime<Utc>,

    pub version: String,
}

#[derive(Clone)]
pub struct StoredParts<R> {
    pub resource: R,
    pub meta: StoredMeta,
}

#[derive(Debug, strum_macros::Display, strum_macros::EnumString, PartialEq)]
#[strum(ascii_case_insensitive)]
pub enum MetaAttr {
    #[strum(to_string = "resourceType")]
    ResourceType,
    #[strum(to_string = "created")]
    Created,
    #[strum(to_string = "lastModified")]
    LastModified,
    #[strum(to_string = "version")]
    Version,
    #[strum(to_string = "location")]
    Location,
}
