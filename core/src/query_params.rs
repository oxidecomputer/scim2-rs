// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[derive(Deserialize, JsonSchema, Clone, Debug)]
pub struct QueryParams {
    pub attributes: Option<String>,
    // TODO: pagination: start_index and count

    // TODO: filter
}
