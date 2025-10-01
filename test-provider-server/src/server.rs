// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;
use scim2_rs::GROUP_URN;
use scim2_rs::LISTRESPONSE_URN;
use scim2_rs::RESOURCETYPE_URN;
use scim2_rs::USER_URN;

#[endpoint {
    method = GET,
    path = "/v2/ResourceTypes"
}]
pub async fn get_resource_types(
    rqctx: RequestContext<Arc<ServerContext>>,
) -> Result<Response<Body>, HttpError> {
    let _apictx = rqctx.context();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
              "schemas": [
                LISTRESPONSE_URN,
              ],
              "totalResults": 2,
              "startIndex": 1,
              "itemsPerPage": 2,
              "Resources": [
                {
                  "id": "User",
                  "name": "User",
                  "description": "User Account",
                  "endpoint": "/Users",
                  "schema": USER_URN
                },
                {
                  "id": "Group",
                  "name": "Group",
                  "description": "Group",
                  "endpoint": "/Groups",
                  "schema": GROUP_URN,
                }
              ]
            }
                         )
            .to_string()
            .into(),
        )
        .unwrap())
}

#[endpoint {
    method = GET,
    path = "/v2/ResourceTypes/User"
}]
pub async fn get_resource_type_user(
    rqctx: RequestContext<Arc<ServerContext>>,
) -> Result<Response<Body>, HttpError> {
    let _apictx = rqctx.context();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
              "schemas": [
                RESOURCETYPE_URN
              ],
              "id": "User",
              "name": "User",
              "description": "User Account",
              "endpoint": "/Users",
              "schema": USER_URN,
            }
            )
            .to_string()
            .into(),
        )
        .unwrap())
}

#[endpoint {
    method = GET,
    path = "/v2/ResourceTypes/Group"
}]
pub async fn get_resource_type_group(
    rqctx: RequestContext<Arc<ServerContext>>,
) -> Result<Response<Body>, HttpError> {
    let _apictx = rqctx.context();

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
              "schemas": [
                RESOURCETYPE_URN
              ],
              "id": "Group",
              "name": "Group",
              "description": "Group",
              "endpoint": "/Groups",
              "schema": GROUP_URN,
            }
            )
            .to_string()
            .into(),
        )
        .unwrap())
}

#[endpoint {
    method = GET,
    path = "/v2/Schemas"
}]
pub async fn get_schemas(
    rqctx: RequestContext<Arc<ServerContext>>,
) -> Result<Response<Body>, HttpError> {
    let _apictx = rqctx.context();

    Ok(Response::builder()
        .status(200)
        .header("Content-TYpe", "application/json")
        .body(
            serde_json::json!(
                {
                  "schemas": [
                    LISTRESPONSE_URN
                  ],
                  "totalResults": 2,
                  "startIndex": 1,
                  "itemsPerPage": 2,
                  "Resources": [
                    {
                      "id": USER_URN,
                      "name": "User",
                      "attributes": [
                        {
                          "name": "userName",
                          "type": "string",
                          "multiValued": false
                        },
                        {
                          "name": "externalId",
                          "type": "string",
                          "multiValued": false
                        },
                        {
                          "name": "active",
                          "type": "boolean",
                          "multiValued": false
                        }
                      ]
                    },
                    {
                      "id": GROUP_URN,
                      "name": "Group",
                      "attributes": [
                        {
                          "name": "displayName",
                          "type": "string",
                          "multiValued": false
                        }
                      ]
                    }
                  ]
                }
            )
            .to_string()
            .into(),
        )
        .unwrap())
}

#[endpoint {
    method = GET,
    path = "/v2/ServiceProviderConfig"
}]
pub async fn get_service_provider_config(
    rqctx: RequestContext<Arc<ServerContext>>,
) -> Result<Response<Body>, HttpError> {
    let _apictx = rqctx.context();

    Ok(Response::builder()
        .status(200)
        .header("Content-TYpe", "application/json")
        .body(
            serde_json::json!(
                {
                  "schemas": [
                    "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"
                  ],
                  "patch": {
                    "supported": true
                  },
                  "bulk": {
                    "supported": false
                  },
                  "filter": {
                    "supported": true
                  },
                  "changePassword": {
                    "supported": false
                  },
                  "sort": {
                    "supported": false
                  },
                  "etag": {
                    "supported": false
                  },
                  "authenticationSchemes": []
                }

            )
            .to_string()
            .into(),
        )
        .unwrap())
}
