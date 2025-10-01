// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

use crate::ResourceType;

pub trait Resource: std::fmt::Debug + Serialize {
    fn id(&self) -> String;
    fn schema() -> String;
    fn resource_type() -> ResourceType;
}
