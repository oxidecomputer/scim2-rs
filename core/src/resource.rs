// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

#[derive(Debug, Copy, Clone, PartialEq, strum_macros::Display)]
pub enum ResourceType {
    User,
    Group,
}

impl ResourceType {
    pub fn urn(&self) -> &str {
        match self {
            ResourceType::User => "urn:ietf:params:scim:schemas:core:2.0:User",
            ResourceType::Group => {
                "urn:ietf:params:scim:schemas:core:2.0:Group"
            }
        }
    }
}

pub trait Resource: std::fmt::Debug + Serialize {
    fn id(&self) -> String;
    fn schema() -> String;
    fn resource_type() -> ResourceType;
}
