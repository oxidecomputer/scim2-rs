// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::str::FromStr;

use super::*;

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
                    Some(dbg!(&query_params).clone()),
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

#[derive(Serialize, JsonSchema, Debug)]
struct ResourceInner {
    #[serde(flatten)]
    resource: serde_json::Map<String, serde_json::Value>,
    schemas: Vec<String>,
}

/// The generic response used to return a single resource
#[derive(Serialize, JsonSchema, Debug)]
pub struct SingleResourceResponse {
    #[serde(flatten)]
    resource: ResourceInner,

    meta: Meta,
}

impl SingleResourceResponse {
    pub fn from_resource<R>(
        resource: R,
        meta: StoredMeta,
        query_params: Option<QueryParams>,
    ) -> Result<Self, Error>
    where
        R: Resource + Serialize,
    {
        let id = resource.id();

        // We have a strongly typed `Resource` but SCIM allows for IdP's to
        // request a subset of fields via attributes so we need to allow for
        // dynamic manipulation.
        let mut obj = serialize_resource_to_object(resource)?;
        let (_meta_attrs, user_group_attrs): (Vec<_>, Vec<_>) = query_params
            .map(|qp| {
                parse_scim_query_attributes(
                    &qp.attributes.unwrap_or_default(),
                    R::resource_type(),
                )
            })
            .unwrap_or_default()
            .into_iter()
            .partition(|attr| matches!(attr, ScimAttribute::Meta(_)));

        // XXX This is a bit of a mess...
        // - certain fields like "id" in a `User` is always required
        // - the schemas field is not tracked anywhere
        // - the meta field is still strucutred rather than being a `Map` we can
        //   mutate
        if !user_group_attrs.is_empty() {
            obj = user_group_attrs
                .into_iter()
                .filter_map(|attr| {
                    obj.get_key_value(&attr.resource_key())
                        .map(|(k, v)| (k.clone(), v.clone()))
                })
                .collect()
        }
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

    status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "scimType")]
    error_type: Option<ErrorType>,

    detail: String,
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

fn parse_scim_query_attributes(
    query_param: &str,
    resource_type: ResourceType,
) -> Vec<ScimAttribute> {
    query_param
        .split(',')
        .flat_map(|qp| ScimAttribute::parse(resource_type, qp))
        .collect()
}

#[derive(Debug, PartialEq)]
enum ScimAttribute {
    User(UserAttr),
    Group(GroupAttr),
    Meta(MetaAttr),
}

impl ScimAttribute {
    /// Parse an attribute based on RFC 7644 Section 3.10
    fn parse(resource_type: ResourceType, raw: &str) -> Option<Self> {
        // All facets (URN, attribute, and sub-attribute name) of the fully
        // encoded attribute name are case insensitive.
        let urn = resource_type.urn().to_lowercase();
        let raw = raw.trim().to_lowercase();

        //{urn}:{Attribute name}.{Sub-Attribute name}.
        // For example, the fully qualified path for a User's givenName is
        // "urn:ietf:params:scim:schemas:core:2.0:User:name.givenName"
        let mut parts = raw.rsplitn(2, ':');
        let obj_path = parts.next()?;

        // Clients MAY omit core schema attribute URN prefixes but SHOULD fully
        // qualify extended attributes with the associated schema extension URN
        // to avoid naming conflicts.
        if let Some(schema) = parts.next() {
            if schema != urn {
                return None;
            }
        }

        // TODO: We should probably log this unexpected case once we have a
        // logger to pass around.
        if parts.next().is_some() {
            return None;
        }

        // Figure out if we have an attribute name and a sub-attribute name
        let mut attr_parts = obj_path.splitn(2, '.');
        let attr = attr_parts.next()?;

        let attribute = match resource_type {
            ResourceType::User => {
                match UserAttr::from_str(attr).ok()? {
                    // This is the only sub-attribute we support
                    UserAttr::Meta => {
                        match attr_parts.next() {
                            // We want some portion of the meta object
                            Some(sub_attr) => {
                                Self::Meta(MetaAttr::from_str(sub_attr).ok()?)
                            }
                            // We want the full meta object
                            None => Self::User(UserAttr::Meta),
                        }
                    }
                    attribute => Self::User(attribute),
                }
            }
            ResourceType::Group => {
                match GroupAttr::from_str(attr).ok()? {
                    // This is the only sub-attribute we support
                    GroupAttr::Meta => {
                        match attr_parts.next() {
                            // We want some portion of the meta object
                            Some(sub_attr) => {
                                Self::Meta(MetaAttr::from_str(sub_attr).ok()?)
                            }
                            // We want the full meta object
                            None => Self::Group(GroupAttr::Meta),
                        }
                    }
                    attribute => Self::Group(attribute),
                }
            }
        };

        Some(attribute)
    }

    fn resource_key(&self) -> String {
        match self {
            ScimAttribute::User(user_attr) => user_attr.to_string(),
            ScimAttribute::Group(group_attr) => group_attr.to_string(),
            ScimAttribute::Meta(meta_attr) => meta_attr.to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{GroupAttr, UserAttr, response::ScimAttribute};

    #[test]
    fn test_user_attribute_parsing() {
        // User field
        assert_eq!(
            ScimAttribute::parse(crate::ResourceType::User, "userName"),
            Some(ScimAttribute::User(UserAttr::Name))
        );

        // User field with urn prefix
        assert_eq!(
            ScimAttribute::parse(
                crate::ResourceType::User,
                "urn:ietf:params:scim:schemas:core:2.0:User:username"
            ),
            Some(ScimAttribute::User(UserAttr::Name))
        );

        // User field with wrong urn prefix
        assert_eq!(
            ScimAttribute::parse(
                crate::ResourceType::User,
                "urn:ietf:params:scim:schemas:core:2.0:Group:name"
            ),
            None
        );

        // User full meta attr requested
        assert_eq!(
            ScimAttribute::parse(crate::ResourceType::User, "meta"),
            Some(ScimAttribute::User(UserAttr::Meta))
        );

        // User sub-meta attr requested
        assert_eq!(
            ScimAttribute::parse(crate::ResourceType::User, "meta.location"),
            Some(ScimAttribute::Meta(crate::MetaAttr::Location))
        );
    }

    #[test]
    fn test_group_attribute_parsing() {
        // Group field -- all lowercase
        assert_eq!(
            ScimAttribute::parse(crate::ResourceType::Group, "displayname"),
            Some(ScimAttribute::Group(GroupAttr::DisplayName))
        );

        // Group field with urn prefix
        assert_eq!(
            ScimAttribute::parse(
                crate::ResourceType::Group,
                "urn:ietf:params:scim:schemas:core:2.0:Group:displayName"
            ),
            Some(ScimAttribute::Group(GroupAttr::DisplayName))
        );

        // Group field with wrong urn prefix
        assert_eq!(
            ScimAttribute::parse(
                crate::ResourceType::Group,
                "urn:ietf:params:scim:schemas:core:2.0:User:displayName"
            ),
            None
        );

        // Group full meta attr requested
        assert_eq!(
            ScimAttribute::parse(crate::ResourceType::Group, "meta"),
            Some(ScimAttribute::Group(GroupAttr::Meta))
        );

        // Group sub-meta attr requested
        assert_eq!(
            ScimAttribute::parse(
                crate::ResourceType::Group,
                "meta.lastmodified"
            ),
            Some(ScimAttribute::Meta(crate::MetaAttr::LastModified))
        );
    }
}
