use schemars::JsonSchema;
use serde::Deserialize;

use crate::{StoredGroup, StoredUser};

#[derive(Debug)]
pub enum PatchRequestError {
    Invalid(String),
    Unsupported(String),
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(tag = "op")]
pub enum PatchOp {
    #[serde(rename = "replace")]
    // NB: replace ops have optional paths but we don't support it yet
    Replace { value: serde_json::Value },
    #[serde(rename = "add")]
    Add { path: Option<String>, value: serde_json::Value },
    #[serde(rename = "remove")]
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
        const URN: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
        match matches!(&self.schemas[..], [val] if val == URN) {
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
        stored_user: &StoredUser,
    ) -> Result<StoredUser, PatchRequestError> {
        self.validate_schema()?;
        let mut updated_user = stored_user.clone();

        for patch_op in &self.operations {
            let PatchOp::Replace { value } = patch_op else {
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

            updated_user.active = change.active;
        }

        Ok(updated_user)
    }

    /// For the given `PatchRequest` attempt to return a new `StoredGroup` after
    /// applying a series of `PatchOp`s to the original object.
    pub fn apply_group_ops(
        &self,
        stored_group: &StoredGroup,
    ) -> Result<StoredGroup, PatchRequestError> {
        self.validate_schema()?;
        let mut updated_group = stored_group.clone();

        for patch_op in &self.operations {
            match patch_op {
                PatchOp::Replace { value } => {
                    apply_group_replace_op(value, &mut updated_group)?
                }
                PatchOp::Add { path, value } => apply_group_add_op(
                    path.as_deref(),
                    value,
                    &mut updated_group,
                )?,
                PatchOp::Remove { path } => {
                    apply_group_remove_op(path, &mut updated_group)?
                }
            }
        }

        Ok(updated_group)
    }
}

fn apply_group_replace_op(
    value: &serde_json::Value,
    group: &mut StoredGroup,
) -> Result<(), PatchRequestError> {
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DisplayName {
        id: String,
        display_name: String,
    }

    let Ok(change) = serde_json::from_value::<DisplayName>(value.clone())
    else {
        return Err(PatchRequestError::Unsupported(
            "only replacing a groups displayName is supported".to_string(),
        ));
    };

    if !group.id.eq_ignore_ascii_case(&change.id) {
        return Err(PatchRequestError::Invalid(format!(
            "unexpected group id {}",
            change.id
        )));
    }
    group.display_name = change.display_name;

    Ok(())
}

fn apply_group_add_op(
    path: Option<&str>,
    value: &serde_json::Value,
    group: &mut StoredGroup,
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

        if group.display_name.eq_ignore_ascii_case(&member.display) {
            return Err(PatchRequestError::Invalid(format!(
                "group add op value display was {} expected {}",
                member.display, group.display_name
            )));
        }

        group.members.push(crate::StoredGroupMember {
            resource_type: crate::ResourceType::User,
            value: member.value,
        });
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
            let path = path.trim_start_matches("members[value eq ");
            let value = path.trim_end_matches(']');

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
    path: &str,
    group: &mut StoredGroup,
) -> Result<(), PatchRequestError> {
    match parse_remove_path(path)? {
        GroupRemoveOp::All => group.members = Vec::new(),
        GroupRemoveOp::Indvidual(value) => {
            if let Some(idx) = group
                .members
                .iter()
                .position(|m| m.value.eq_ignore_ascii_case(&value))
            {
                group.members.swap_remove(idx);
            }
        }
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::PatchRequest;

    #[test]
    fn test_parse_user_active_replace_op() {
        let json = json!(
            {
              "schemas": [
                "urn:ietf:params:scim:api:messages:2.0:PatchOp"
              ],
              "Operations": [
                {
                  "op": "replace",
                  "value": {
                    "active": false
                  }
                }
              ]
            }
        );

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }

    #[test]
    fn test_parse_group_displayname_replace_op() {
        let json = json!(
            {
              "schemas": [
                "urn:ietf:params:scim:api:messages:2.0:PatchOp"
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
            }
        );

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }

    #[test]
    fn test_parse_group_membership_ops() {
        let json = json!(
            {
              "schemas": [
                "urn:ietf:params:scim:api:messages:2.0:PatchOp"
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
            }
        );

        serde_json::from_value::<PatchRequest>(json).unwrap();
    }
}
