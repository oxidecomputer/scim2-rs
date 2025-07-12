// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[endpoint {
    method = GET,
    path = "/v2/Groups"
}]
pub async fn list_groups(
    _rqctx: RequestContext<Arc<ServerContext>>,
    _query_params: Query<scim2_rs::QueryParams>,
) -> Result<Response<Body>, HttpError> {
    unimplemented!()
}

#[derive(Deserialize, JsonSchema)]
struct GetGroupPathParam {
    _group_id: String,
}

#[endpoint {
    method = GET,
    path = "/v2/Groups/{group_id}"
}]
pub async fn get_group(
    _rqctx: RequestContext<Arc<ServerContext>>,
    _path_param: Path<GetGroupPathParam>,
    _query_params: Query<scim2_rs::QueryParams>,
) -> Result<Response<Body>, HttpError> {
    unimplemented!()
}

#[endpoint {
    method = POST,
    path = "/v2/Groups",
}]
pub async fn create_group(
    _rqctx: RequestContext<Arc<ServerContext>>,
    _new_group: TypedBody<scim2_rs::CreateGroupRequest>,
) -> Result<Response<Body>, HttpError> {
    unimplemented!()
}

#[derive(Deserialize, JsonSchema)]
pub struct PutGroupPathParam {
    _group_id: String,
}

#[endpoint {
    method = PUT,
    path = "/v2/Groups/{group_id}"
}]
pub async fn put_group(
    _rqctx: RequestContext<Arc<ServerContext>>,
    _path_param: Path<PutGroupPathParam>,
    _updated_group: TypedBody<scim2_rs::CreateGroupRequest>,
) -> Result<Response<Body>, HttpError> {
    unimplemented!()
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteGroupPathParam {
    _group_id: String,
}

#[endpoint {
    method = DELETE,
    path = "/v2/Groups/{group_id}"
}]
pub async fn delete_group(
    _rqctx: RequestContext<Arc<ServerContext>>,
    _path_param: Path<DeleteGroupPathParam>,
) -> Result<Response<Body>, HttpError> {
    unimplemented!()
}
