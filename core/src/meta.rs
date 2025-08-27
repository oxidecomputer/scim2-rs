// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Resource;

#[derive(Deserialize, Serialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub resource_type: String,

    pub created: DateTime<Utc>,

    pub last_modified: DateTime<Utc>,

    pub version: String,

    pub location: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StoredMeta {
    pub created: DateTime<Utc>,

    pub last_modified: DateTime<Utc>,

    pub version: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StoredParts<R: Resource> {
    pub resource: R,
    pub meta: StoredMeta,
}

impl From<Meta> for StoredMeta {
    fn from(m: Meta) -> StoredMeta {
        StoredMeta {
            created: m.created,
            last_modified: m.last_modified,
            version: m.version,
        }
    }
}
