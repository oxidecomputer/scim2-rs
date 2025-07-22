// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;

/// The generic response used to return a list of resources
#[derive(Deserialize, Serialize, JsonSchema)]
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
    pub resources: Vec<serde_json::Map<String, serde_json::Value>>,
}

impl ListResponse {
    pub fn from_resources<R, S>(
        resources: Vec<S>,
        query_params: QueryParams,
    ) -> Result<Self, Error>
    where
        R: Resource,
        S: Into<StoredParts<R>>,
    {
        let schemas = vec![String::from(
            "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        )];

        // TODO pagination should have happened before this, but fill in
        // start_index and items_per_page below based on query_params

        let resources = resources
            .into_iter()
            .map(|s| {
                let StoredParts { resource, meta } = s.into();
                SingleResourceResponse::from_resource(
                    resource,
                    meta,
                    Some(query_params.clone()),
                )
            })
            .collect::<Result<Vec<_>, Error>>()?
            .into_iter()
            .map(serialize_resource_to_object)
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(ListResponse {
            schemas,
            total_results: resources.len(),
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

#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct ResourceInner {
    #[serde(flatten)]
    pub resource: serde_json::Map<String, serde_json::Value>,

    pub schemas: Vec<String>,
}

/// The generic response used to return a single resource
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct SingleResourceResponse {
    #[serde(flatten)]
    pub resource: ResourceInner,

    pub meta: Meta,
}

impl SingleResourceResponse {
    pub fn from_resource<R>(
        resource: R,
        meta: StoredMeta,
        _query_params: Option<QueryParams>,
    ) -> Result<Self, Error>
    where
        R: Resource + Serialize,
    {
        let id = resource.id();

        // We have a strongly typed `Resource` but SCIM allows for IdP's to
        // request a subset of fields via attributes so we need to allow for
        // dynamic manipulation.
        let obj = serialize_resource_to_object(resource)?;

        let resource =
            ResourceInner { resource: obj, schemas: vec![R::schema()] };

        Ok(SingleResourceResponse {
            resource,
            meta: Meta {
                resource_type: R::resource_type(),
                created: meta.created,
                last_modified: meta.last_modified,
                version: meta.version,
                location: format!(
                    "http://127.0.0.1:4567/v2/{}s/{}",
                    R::resource_type(),
                    id
                ),
            },
        })
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
#[derive(Deserialize, Serialize, JsonSchema, Debug, PartialEq)]
pub enum ErrorType {
    #[serde(rename = "invalidFilter")]
    InvalidFilter,

    #[serde(rename = "uniqueness")]
    Uniqueness,
}

/// The SCIM error format is specified in RFC 7644, section 3.12
#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct Error {
    pub schemas: Vec<String>,

    pub status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "scimType")]
    pub error_type: Option<ErrorType>,

    pub detail: String,
}

impl Error {
    fn new(
        status: StatusCode,
        error_type: Option<ErrorType>,
        detail: String,
    ) -> Self {
        Self {
            schemas: vec![String::from(
                "urn:ietf:params:scim:api:messages:2.0:Error",
            )],
            status: status.as_str().to_string(),
            error_type,
            detail,
        }
    }

    pub fn invalid_filter(detail: String) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            Some(ErrorType::InvalidFilter),
            detail,
        )
    }

    pub fn not_found(id: String) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            None,
            format!("Resource {id} not found"),
        )
    }

    pub fn conflict(identifier: String) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            Some(ErrorType::Uniqueness),
            format!("Resource matching {identifier} exists already"),
        )
    }

    pub fn internal_error(detail: String) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, None, detail)
    }

    pub fn status(&self) -> Result<u16, std::num::ParseIntError> {
        self.status.parse()
    }

    pub fn to_http_response(self) -> Result<Response<Body>, http::Error> {
        let status = match self.status() {
            Ok(status) => status,

            Err(e) => {
                return Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(
                    serde_json::json!(
                        {
                        "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                        "status": 500,
                        "detail": format!("parsing {} as u16 failed: {e}", self.status),
                        }
                    )
                    .to_string()
                    .into(),
                )
            }
        };

        match serde_json::to_string(&self) {
            Ok(serialized) => Response::builder()
                .status(status)
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

pub fn deleted_http_response() -> Result<Response<Body>, Error> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .map_err(|e| Error::internal_error(format!("{e}")))
}

/// Convert a `Resource` to a more dynamic `serde_json::Map`
fn serialize_resource_to_object<R>(
    resource: R,
) -> Result<serde_json::Map<String, serde_json::Value>, Error>
where
    R: Serialize + std::fmt::Debug,
{
    let value = match serde_json::to_value(&resource) {
        Ok(value) => value,
        Err(e) => {
            return Err(Error::internal_error(format!(
                "failed to serialize resource {resource:?} to JSON: {e}"
            )));
        }
    };

    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Err(Error::internal_error(format!(
            "resource {resource:?} is not a JSON object"
        ))),
    }
}
