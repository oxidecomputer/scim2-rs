// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[endpoint {
    method = GET,
    path = "/v2/Groups"
}]
pub async fn list_groups(
    rqctx: RequestContext<Arc<ServerContext>>,
    query_params: Query<scim2_rs::QueryParams>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let query_params = query_params.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.list_groups(query_params).await {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}

#[derive(Deserialize, JsonSchema)]
struct GetGroupPathParam {
    group_id: String,
}

#[endpoint {
    method = GET,
    path = "/v2/Groups/{group_id}"
}]
pub async fn get_group(
    rqctx: RequestContext<Arc<ServerContext>>,
    path_param: Path<GetGroupPathParam>,
    query_params: Query<scim2_rs::QueryParams>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let query_params = query_params.into_inner();
    let path_param = path_param.into_inner();

    let result: Result<Response<Body>, http::Error> = match apictx
        .provider
        .get_group_by_id(query_params, path_param.group_id)
        .await
    {
        Ok(response) => response.to_http_response(),
        Err(error) => error.to_http_response(),
    };

    result.map_err(|e| HttpError::from(e))
}

#[endpoint {
    method = POST,
    path = "/v2/Groups",
}]
pub async fn create_group(
    rqctx: RequestContext<Arc<ServerContext>>,
    body: TypedBody<scim2_rs::CreateGroupRequest>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let request = body.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.create_group(request).await {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}

#[derive(Deserialize, JsonSchema)]
pub struct PutGroupPathParam {
    group_id: String,
}

#[endpoint {
    method = PUT,
    path = "/v2/Groups/{group_id}"
}]
pub async fn put_group(
    rqctx: RequestContext<Arc<ServerContext>>,
    path_param: Path<PutGroupPathParam>,
    body: TypedBody<scim2_rs::CreateGroupRequest>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let path_param = path_param.into_inner();
    let request = body.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.replace_group(path_param.group_id, request).await
        {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteGroupPathParam {
    group_id: String,
}

#[endpoint {
    method = DELETE,
    path = "/v2/Groups/{group_id}"
}]
pub async fn delete_group(
    rqctx: RequestContext<Arc<ServerContext>>,
    path_param: Path<DeleteGroupPathParam>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let path_param = path_param.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.delete_group(path_param.group_id).await {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}
