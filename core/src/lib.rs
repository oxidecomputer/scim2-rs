// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use dropshot::Body;
use http::Response;
use http::StatusCode;
use iddqd::IdOrdItem;
use iddqd::IdOrdMap;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

mod group;
mod in_memory_provider_store;
mod meta;
mod patch;
mod provider;
mod provider_store;
mod query_params;
mod resource;
mod response;
mod user;

pub use group::*;
pub use in_memory_provider_store::*;
pub use meta::*;
pub use patch::*;
pub use provider::*;
pub use provider_store::*;
pub use query_params::*;
pub use resource::*;
pub use response::*;
pub use user::*;

/// Skip serializing if optional list is None or empty
pub fn skip_serializing_list<T>(members: &Option<Vec<T>>) -> bool {
    match members {
        None => true,
        Some(v) => v.is_empty(),
    }
}

/// Skip serializing if optional list is None or empty for IdOrdMap
pub fn skip_serializing_list_map<T>(members: &Option<IdOrdMap<T>>) -> bool
where
    T: IdOrdItem,
{
    match members {
        None => true,
        Some(v) => v.is_empty(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub enum ResourceType {
    User,
    Group,
}

impl std::str::FromStr for ResourceType {
    type Err = String;

    fn from_str(r: &str) -> Result<Self, Self::Err> {
        match r.to_lowercase().as_str() {
            "user" => Ok(ResourceType::User),
            "group" => Ok(ResourceType::Group),
            _ => Err(format!("{r} not a valid resource type")),
        }
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ResourceType::User => {
                write!(f, "User")
            }

            ResourceType::Group => {
                write!(f, "Group")
            }
        }
    }
}
