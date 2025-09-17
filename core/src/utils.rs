// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use iddqd::IdOrdItem;
use iddqd::IdOrdMap;
use schemars::JsonSchema;
use serde::Serialize;

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

// We are emitting upper case values here since all of the examples in RFC 7643
// use upper case even though the spec says that this value is not to be
// interpreted as caseExact.
//
//   {
//     "name" : "ResourceType",
//     "attributes" : [
//       {
//         "name" : "id",
//         "type" : "string",
//         "caseExact" : false,
//       },
//     ......snip......
//    }
//
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
