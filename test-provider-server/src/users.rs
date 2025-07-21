// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

#[endpoint {
    method = GET,
    path = "/v2/Users"
}]
pub async fn list_users(
    rqctx: RequestContext<Arc<ServerContext>>,
    query_params: Query<scim2_rs::QueryParams>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let query_params = query_params.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.list_users(query_params).await {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}

#[derive(Deserialize, JsonSchema)]
pub struct GetUserPathParam {
    user_id: String,
}

#[endpoint {
    method = GET,
    path = "/v2/Users/{user_id}"
}]
pub async fn get_user(
    rqctx: RequestContext<Arc<ServerContext>>,
    path_param: Path<GetUserPathParam>,
    query_params: Query<scim2_rs::QueryParams>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let query_params = query_params.into_inner();
    let path_param = path_param.into_inner();

    let result: Result<Response<Body>, http::Error> = match apictx
        .provider
        .get_user_by_id(query_params, path_param.user_id)
        .await
    {
        Ok(response) => response.to_http_response(),
        Err(error) => error.to_http_response(),
    };

    result.map_err(|e| HttpError::from(e))
}

#[endpoint {
    method = POST,
    path = "/v2/Users",
}]
pub async fn create_user(
    rqctx: RequestContext<Arc<ServerContext>>,
    body: TypedBody<scim2_rs::CreateUserRequest>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let request = body.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.create_user(request).await {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}

#[derive(Deserialize, JsonSchema)]
pub struct PutUserPathParam {
    user_id: String,
}

#[endpoint {
    method = PUT,
    path = "/v2/Users/{user_id}"
}]
pub async fn put_user(
    rqctx: RequestContext<Arc<ServerContext>>,
    path_param: Path<PutUserPathParam>,
    body: TypedBody<scim2_rs::CreateUserRequest>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let path_param = path_param.into_inner();
    let request = body.into_inner();

    let result: Result<Response<Body>, http::Error> = match apictx
        .provider
        .replace_user(path_param.user_id, request)
        .await
    {
        Ok(response) => response.to_http_response(),
        Err(error) => error.to_http_response(),
    };

    result.map_err(|e| HttpError::from(e))
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteUserPathParam {
    user_id: String,
}

#[endpoint {
    method = DELETE,
    path = "/v2/Users/{user_id}"
}]
pub async fn delete_user(
    rqctx: RequestContext<Arc<ServerContext>>,
    path_param: Path<DeleteUserPathParam>,
) -> Result<Response<Body>, HttpError> {
    let apictx = rqctx.context();
    let path_param = path_param.into_inner();

    let result: Result<Response<Body>, http::Error> =
        match apictx.provider.delete_user(path_param.user_id).await {
            Ok(response) => response.to_http_response(),
            Err(error) => error.to_http_response(),
        };

    result.map_err(|e| HttpError::from(e))
}
