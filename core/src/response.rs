// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

use std::collections::BTreeMap;

/// The generic response used to return a list of resources
#[derive(Serialize, JsonSchema)]
pub struct ListResponse {
    pub schemas: Vec<String>,

    #[serde(rename = "totalResults")]
    pub total_results: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "startIndex")]
    pub start_index: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: Option<usize>,

    #[serde(rename = "Resources")]
    pub resources: Vec<BTreeMap<String, serde_json::value::Value>>,
    // TODO: meta
}

impl ListResponse {
    pub fn from_resources<T>(
        v: Vec<T>,
        _query_params: QueryParams,
    ) -> Result<Self, Error>
    where
        T: Resource + Serialize,
    {
        let schemas = vec![String::from(
            "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        )];

        let serialized = serde_json::to_string(&v)
            .map_err(|e| Error::internal_error(format!("{e}")))?;
        let resources = serde_json::from_str(&serialized)
            .map_err(|e| Error::internal_error(format!("{e}")))?;

        // TODO: filter by attributes
        // pagination should have happened before this, but fill in start_index
        // and items_per_page below

        Ok(ListResponse {
            schemas,
            total_results: v.len(),
            start_index: None,
            items_per_page: None,
            resources,
        })
    }

    pub fn to_http_response(self) -> Result<Response<Body>, http::Error> {
        match serde_json::to_string(&self) {
            Ok(serialized) => Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(serialized.into()),

            Err(e) => Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!(
                        {
                        "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                        "status": 500,
                        "detail": format!("serializing error failed: {e}"),
                        }
                    )
                    .to_string()
                    .into(),
                ),
        }
    }
}

/// The generic response used to return a single resource
#[derive(Serialize, JsonSchema)]
pub struct SingleResourceResponse {
    #[serde(flatten)]
    resource: BTreeMap<String, serde_json::value::Value>,
    // TODO: meta
}

impl SingleResourceResponse {
    pub fn from_resource<T>(
        v: T,
        _query_params: Option<QueryParams>,
    ) -> Result<Self, Error>
    where
        T: Resource + Serialize,
    {
        let serialized = serde_json::to_string(&v)
            .map_err(|e| Error::internal_error(format!("{e}")))?;

        let mut resource: BTreeMap<String, serde_json::value::Value> =
            serde_json::from_str(&serialized)
                .map_err(|e| Error::internal_error(format!("{e}")))?;

        resource.insert(String::from("schema"), T::schema().into());

        Ok(SingleResourceResponse { resource })
    }

    pub fn to_http_response(
        self,
        status_code: StatusCode,
    ) -> Result<Response<Body>, http::Error> {
        match serde_json::to_string(&self) {
            Ok(serialized) => Response::builder()
                .status(status_code)
                .header("Content-Type", "application/json")
                .body(serialized.into()),

            Err(e) => Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!(
                        {
                        "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                        "status": 500,
                        "detail": format!("serializing error failed: {e}"),
                        }
                    )
                    .to_string()
                    .into(),
                ),
        }
    }
}

/// The SCIM error types specified in RFC 7644, section 3.12
// RFC 7644, section 3.12:  HTTP Status and Error Response Handling
#[derive(Serialize, JsonSchema, Debug)]
pub enum ErrorType {
    #[serde(rename = "invalidFilter")]
    InvalidFilter,

    #[serde(rename = "uniqueness")]
    Uniqueness,
}

/// The SCIM error format is specified in RFC 7644, section 3.12
#[derive(Serialize, JsonSchema, Debug)]
pub struct Error {
    schemas: Vec<String>,

    status: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "scimType")]
    error_type: Option<ErrorType>,

    detail: String,
}

impl Error {
    pub fn new(
        status: u16,
        error_type: Option<ErrorType>,
        detail: String,
    ) -> Self {
        Self {
            schemas: vec![String::from(
                "urn:ietf:params:scim:api:messages:2.0:Error",
            )],
            status,
            error_type,
            detail,
        }
    }

    pub fn invalid_filter(detail: String) -> Self {
        Self::new(400, Some(ErrorType::InvalidFilter), detail)
    }

    pub fn not_found(id: String) -> Self {
        Self::new(404, None, format!("Resource {id} not found"))
    }

    pub fn conflict(identifier: String) -> Self {
        Self::new(
            409,
            Some(ErrorType::Uniqueness),
            format!("Resource matching {identifier} exists already"),
        )
    }

    pub fn internal_error(detail: String) -> Self {
        Self::new(500, None, detail)
    }

    pub fn to_http_response(self) -> Result<Response<Body>, http::Error> {
        match serde_json::to_string(&self) {
            Ok(serialized) => Response::builder()
                .status(self.status)
                .header("Content-Type", "application/json")
                .body(serialized.into()),

            Err(e) => Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!(
                        {
                        "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                        "status": 500,
                        "detail": format!("serializing error failed: {e}"),
                        }
                    )
                    .to_string()
                    .into(),
                ),
        }
    }
}
