// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use dropshot::Body;
use http::{Response, StatusCode, header};
use schemars::{
    JsonSchema, SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject},
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    Meta, PatchRequestError, QueryParams, Resource, StoredMeta, StoredParts,
};

const CONTENT_TYPE_SCIM_JSON: &str = "application/scim+json";

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
    pub fn from_resources<R>(
        resources: Vec<StoredParts<R>>,
        query_params: QueryParams,
    ) -> Result<Self, Error>
    where
        R: Resource,
    {
        let schemas = vec![String::from(
            "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        )];

        // TODO pagination should have happened before this, but fill in
        // start_index and items_per_page below based on query_params

        let resources = resources
            .into_iter()
            .map(|s| {
                let StoredParts { resource, meta } = s;
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
        value_to_http_response(
            StatusCode::OK,
            &self,
            "serializing list response failed",
        )
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
                resource_type: R::resource_type().to_string(),
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
        value_to_http_response(
            status_code,
            &self,
            "serializing resource failed",
        )
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

    #[serde(rename = "invalidSyntax")]
    InvalidSyntax,

    #[serde(rename = "mutability")]
    Mutability,
}

fn status_to_string<S>(
    status: &StatusCode,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(status.as_str())
}

fn string_to_status<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse::<StatusCode>().map_err(serde::de::Error::custom)
}

fn status_code_schema(_: &mut SchemaGenerator) -> Schema {
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        ..Default::default()
    }
    .into()
}

/// The SCIM error format is specified in RFC 7644, section 3.12
#[derive(Deserialize, Serialize, JsonSchema, Debug, PartialEq)]
pub struct Error {
    pub schemas: Vec<String>,

    #[serde(serialize_with = "status_to_string")]
    #[serde(deserialize_with = "string_to_status")]
    #[schemars(schema_with = "status_code_schema")]
    pub status: StatusCode,

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
            status,
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

    pub fn invalid_syntax(detail: String) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            Some(ErrorType::InvalidSyntax),
            detail,
        )
    }

    pub fn not_implemented(detail: String) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, None, detail)
    }

    pub fn mutability(detail: String) -> Self {
        Self::new(StatusCode::BAD_REQUEST, Some(ErrorType::Mutability), detail)
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn to_http_response(self) -> Result<Response<Body>, http::Error> {
        value_to_http_response(self.status, &self, "serializing error failed")
    }
}

impl From<PatchRequestError> for Error {
    fn from(value: PatchRequestError) -> Self {
        match value {
            PatchRequestError::Invalid(detail) => Error::invalid_syntax(detail),
            PatchRequestError::Unsupported(detail) => {
                Error::not_implemented(detail)
            }
        }
    }
}

pub fn value_to_http_response<S: Serialize>(
    status_code: StatusCode,
    value: &S,
    error_context: &str,
) -> Result<Response<Body>, http::Error> {
    match serde_json::to_string(value) {
        Ok(serialized) => Response::builder()
            .status(status_code)
            .header(header::CONTENT_TYPE, CONTENT_TYPE_SCIM_JSON)
            .body(serialized.into()),

        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header(header::CONTENT_TYPE, CONTENT_TYPE_SCIM_JSON)
            .body(
                serde_json::json!(
                    {
                    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                    "status": 500,
                    "detail": format!("{error_context}: {e}"),
                    }
                )
                .to_string()
                .into(),
            ),
    }
}

pub fn deleted_http_response() -> Result<Response<Body>, Error> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(header::CONTENT_TYPE, CONTENT_TYPE_SCIM_JSON)
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
