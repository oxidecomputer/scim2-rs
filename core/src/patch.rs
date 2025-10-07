// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use iddqd::IdOrdMap;
use schemars::JsonSchema;
use serde::Deserialize;
use slog::Logger;
use slog::info;
use unicase::UniCase;

use crate::Group;
use crate::GroupMember;
use crate::PATCHOP_URN;
use crate::StoredParts;
use crate::User;
use crate::utils::ResourceType;

#[derive(Debug)]
pub enum PatchRequestError {
    Invalid(String),
    Unsupported(String),
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(tag = "op", rename_all = "camelCase")]
enum PatchOp {
    Replace { path: Option<String>, value: serde_json::Value },
    Add { path: Option<String>, value: serde_json::Value },
    Remove { path: String },
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PatchRequest {
    schemas: Vec<String>,
    #[serde(rename = "Operations")]
    operations: Vec<PatchOp>,
}

impl PatchRequest {
    /// Ensure that the parsed `PatchRequest` contians the expected schema
    /// field.
    fn validate_schema(&self) -> Result<(), PatchRequestError> {
        match matches!(&self.schemas[..], [val] if val == PATCHOP_URN) {
            true => Ok(()),
            false => Err(PatchRequestError::Invalid(format!(
                "invalid patch schema {:?}",
                self.schemas
            ))),
        }
    }

    /// For the given `PatchRequest` attempt to return a new `StoredUser` after
    /// applying a series of `PatchOp`s to the original object.
    pub fn apply_user_ops(
        &self,
        log: &Logger,
        stored_user: &StoredParts<User>,
    ) -> Result<StoredParts<User>, PatchRequestError> {
        self.validate_schema()?;
        let mut updated_user = stored_user.clone();

        // RFC 7644 3.5.2
        //
        // Each PATCH operation represents a single action to be applied to
        // the same SCIM resource specified by the request URI.  Operations
        // are applied sequentially in the order they appear in the array.
        // Each operation in the sequence is applied to the target resource;
        // the resulting resource becomes the target of the next operation.
        // Evaluation continues until all operations are successfully applied or
        // until an error condition is encountered.
        for patch_op in &self.operations {
            let PatchOp::Replace { path: _, value } = patch_op else {
                return Err(PatchRequestError::Unsupported(
                    "only the replace op is supported for users".to_string(),
                ));
            };

            #[derive(Debug, Deserialize)]
            struct Active {
                active: bool,
            }

            let Ok(change) = serde_json::from_value::<Active>(value.clone())
            else {
                return Err(PatchRequestError::Unsupported(
                    "only replacing the active property is supported"
                        .to_string(),
                ));
            };

            info!(
              log,
              "PatchOp setting user active property";
              "user" => ?stored_user.resource.id,
              "old" => ?stored_user.resource.active,
              "new" => ?change.active,
            );
            updated_user.resource.active = Some(change.active);
        }

        Ok(updated_user)
    }

    /// For the given `PatchRequest` attempt to return a new `StoredGroup` after
    /// applying a series of `PatchOp`s to the original object.
    pub fn apply_group_ops(
        &self,
        log: &Logger,
        stored_group: &StoredParts<Group>,
    ) -> Result<StoredParts<Group>, PatchRequestError> {
        self.validate_schema()?;
        let mut updated_group = stored_group.clone();

        // RFC 7644 3.5.2
        //
        // Each PATCH operation represents a single action to be applied to
        // the same SCIM resource specified by the request URI.  Operations
        // are applied sequentially in the order they appear in the array.
        // Each operation in the sequence is applied to the target resource;
        // the resulting resource becomes the target of the next operation.
        // Evaluation continues until all operations are successfully applied or
        // until an error condition is encountered.
        for patch_op in &self.operations {
            match patch_op {
                PatchOp::Replace { path, value } => apply_group_replace_op(
                    log,
                    path.as_ref(),
                    value,
                    &mut updated_group,
                )?,
                PatchOp::Add { path, value } => apply_group_add_op(
                    log,
                    path.as_deref(),
                    value,
                    &mut updated_group,
                )?,
                PatchOp::Remove { path } => {
                    apply_group_remove_op(log, path, &mut updated_group)?
                }
            }
        }

        Ok(updated_group)
    }
}

fn apply_group_replace_op(
    log: &Logger,
    path: Option<&String>,
    value: &serde_json::Value,
    group: &mut StoredParts<Group>,
) -> Result<(), PatchRequestError> {
    match path {
        // Validate that we are replacing a groups members
        Some(path) => {
            #[derive(Debug, Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Member {
                value: String,
            }

            if !path.eq_ignore_ascii_case("members") {
                return Err(PatchRequestError::Unsupported(
                    "only replacing a groups members path is supported"
                        .to_string(),
                ));
            }

            let Ok(patch_members) =
                serde_json::from_value::<Vec<Member>>(value.clone())
            else {
                return Err(PatchRequestError::Invalid(
                    "members being replaced in a group require a value field"
                        .to_string(),
                ));
            };

            let new_members: IdOrdMap<GroupMember> = patch_members
                .into_iter()
                .map(|member| GroupMember {
                    resource_type: Some(ResourceType::User.to_string()),
                    value: Some(member.value),
                })
                .collect();

            info!(log,
                "PatchOp replacing group members";
                "group" => %group.resource.id,
                "old" => ?group.resource.members,
                "new" => ?new_members,
            );

            group.resource.members =
                (!new_members.is_empty()).then_some(new_members);
        }

        // RFC 7664 § 3.5.2.3:
        //
        // If the "path" parameter is omitted, the target is assumed to be the
        // resource itself. In this case, the "value" attribute SHALL contain a
        // list of one or more attributes that are to be replaced.
        None => {
            #[derive(Debug, Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct DisplayName {
                id: String,
                display_name: String,
            }

            // We currently only support changing a display name
            let Ok(change) =
                serde_json::from_value::<DisplayName>(value.clone())
            else {
                return Err(PatchRequestError::Unsupported(
                    "only replacing a displayName is supported when not
                     including a path"
                        .to_string(),
                ));
            };

            if !group.resource.id.eq_ignore_ascii_case(&change.id) {
                return Err(PatchRequestError::Invalid(format!(
                    "unexpected group id {}",
                    change.id
                )));
            }

            group.resource.display_name = change.display_name;
        }
    }

    Ok(())
}

fn apply_group_add_op(
    log: &Logger,
    path: Option<&str>,
    value: &serde_json::Value,
    group: &mut StoredParts<Group>,
) -> Result<(), PatchRequestError> {
    let Some(_path) = path.filter(|p| p.eq_ignore_ascii_case("members")) else {
        return Err(PatchRequestError::Invalid(
            "group add op must provide members as the path".to_string(),
        ));
    };

    let serde_json::Value::Array(members) = value else {
        return Err(PatchRequestError::Invalid(
            "group add op value must be an array of members".to_string(),
        ));
    };

    for member in members {
        #[derive(Debug, Deserialize)]
        struct Member {
            value: String,
            display: String,
        }
        let Ok(member) = serde_json::from_value::<Member>(member.clone())
        else {
            return Err(PatchRequestError::Invalid(
                "group add op member value expected".to_string(),
            ));
        };

        if group.resource.display_name.eq_ignore_ascii_case(&member.display) {
            return Err(PatchRequestError::Invalid(format!(
                "group add op value display was {} expected {}",
                member.display, group.resource.display_name
            )));
        }

        // 3.5.2.1.  Add Operation
        //
        // If the target location already contains the value specified, no
        // changes SHOULD be made to the resource, and a success response
        // SHOULD be returned.  Unless other operations change the resource,
        // this operation SHALL NOT change the modify timestamp of the
        // resource.
        if let Some(old_member) =
            group.resource.members.get_or_insert_default().insert_overwrite(
                GroupMember {
                    resource_type: Some(ResourceType::User.to_string()),
                    value: Some(member.value),
                },
            )
        {
            // We are replacing an existing value
            info!(log,
                "PatchOp adding existing group member";
                "group" => %group.resource.id,
                "group_member" => ?old_member,
            )
        };
    }

    Ok(())
}

enum GroupRemoveOp {
    All,
    Indvidual(String),
}

fn parse_remove_path(path: &str) -> Result<GroupRemoveOp, PatchRequestError> {
    let path = path.trim().to_lowercase();
    match path.as_str() {
        "members" => Ok(GroupRemoveOp::All),
        path => {
            let value = path
                .strip_prefix("members[value eq ")
                .and_then(|p| p.strip_suffix(']'))
                .ok_or(PatchRequestError::Invalid(
                    "path should be specified as members[value eq \"<VALUE>\"]"
                        .to_string(),
                ))?;

            // The value portion of the expression must be a string wrapped in
            // quotations
            let is_quoted_value = |v: &str| -> Option<String> {
                if v.len() > 2 && v.starts_with('"') && v.ends_with('"') {
                    return Some(v[1..v.len() - 1].to_string());
                }
                None
            };

            is_quoted_value(value)
                .map(GroupRemoveOp::Indvidual)
                .ok_or(PatchRequestError::Invalid(
                "individual group remove op must contain a quoted value in the \
                    path".to_string(),
            ))
        }
    }
}

fn apply_group_remove_op(
    log: &Logger,
    path: &str,
    group: &mut StoredParts<Group>,
) -> Result<(), PatchRequestError> {
    match parse_remove_path(path)? {
        GroupRemoveOp::All => group.resource.members = Some(IdOrdMap::new()),
        GroupRemoveOp::Indvidual(value) => {
            let groups = group.resource.members.get_or_insert_default();

            // 3.5.2.2.  Remove Operation
            //
            // If the user was not a member of this group, no changes should be made
            // to the resource, and a success response should be returned.
            match groups.remove(&Some(UniCase::new(value.as_str()))) {
                Some(removed) => info!(
                    log,
                    "PatchOp removed member from group";
                    "group" => %group.resource.id,
                    "member" => ?removed,
                ),
                None => {
                    info!(
                        log,
                        "PatchOp attempted to remove non group member";
                        "group" => %group.resource.id,
                        "member" => %value
                    );
                }
            };
        }
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{
        PatchRequest,
        patch::{GroupRemoveOp, PATCHOP_URN, parse_remove_path},
    };

    #[test]
    fn test_parse_user_active_replace_op() {
        let json = json!({
          "schemas": [
            PATCHOP_URN
          ],
          "Operations": [
            {
              "op": "replace",
              "value": {
                "active": false
              }
            }
          ]
        });

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }

    #[test]
    fn test_parse_group_displayname_replace_op() {
        let json = json!({
          "schemas": [
            PATCHOP_URN
          ],
          "Operations": [
            {
              "op": "replace",
              "value": {
                "id": "abf4dd94-a4c0-4f67-89c9-76b03340cb9b",
                "displayName": "Test SCIMv2"
              }
            }
          ]
        });

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }

    #[test]
    fn test_parse_group_members_replace_op() {
        let json = json!({
          "schemas": [
            PATCHOP_URN
          ],
          "Operations": [
            {
              "op": "replace",
              "path": "members",
              "value": [
                {
                  "value": "abf4dd94-a4c0-4f67-89c9-76b03340cb9b",
                  "display": "dakota@example.com"
                }
              ]
            }
          ]
        });

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }

    #[test]
    fn test_parse_group_membership_ops() {
        let json = json!({
          "schemas": [
            PATCHOP_URN
          ],
          "Operations": [
            {
              "op": "remove",
              "path": "members[value eq \"89bb1940-b905-4575-9e7f-6f887cfb368e\"]"
            },
            {
              "op": "add",
              "path": "members",
              "value": [
                {
                  "value": "23a35c27-23d3-4c03-b4c5-6443c09e7173",
                  "display": "test.user@okta.local"
                }
              ]
            }
          ]
        });

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }

    #[test]
    fn test_parse_remove_path() {
        // The value we expect an IdP to send
        let user_id = "89bb1940-b905-4575-9e7f-6f887cfb368e".to_string();
        let path = format!("members[value eq \"{user_id}\"]");
        assert!(matches!(
            parse_remove_path(&path).unwrap(),
            GroupRemoveOp::Indvidual(value) if value == user_id,
        ));

        // The value we expec an Idp to send but with mixed case
        let user_id = "89bb1940-b905-4575-9e7f-6f887cfb368e".to_string();
        let path = format!("MemBers[value EQ \"{user_id}\"]");
        assert!(matches!(
            parse_remove_path(&path).unwrap(),
            GroupRemoveOp::Indvidual(value) if value == user_id,
        ));

        // Remove all members
        let path = "members";
        assert!(
            matches!(parse_remove_path(path).unwrap(), GroupRemoveOp::All,)
        );

        // An unsupported value
        let path = "members[value eq \"2819c223-7f76-453a-919d-413861904646\"].displayName";
        assert!(parse_remove_path(path).is_err());

        // An unsupported value
        let path = "addresses[type eq \"work\"]";
        assert!(parse_remove_path(path).is_err());
    }
}
