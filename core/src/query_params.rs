// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema, Clone)]
pub struct QueryParams {
    // TODO: attributes

    // TODO: pagination: start_index and count
    pub filter: Option<String>,
}

impl QueryParams {
    pub fn filter(&self) -> Option<FilterOp> {
        self.filter.as_ref().map(|f| parse_filter_param(f))
    }
}

// These filter operations are exactly what Okta uses rather than implementing
// the full filtering spec found in RFC 7644 Section 3.4.2.2.
#[derive(Debug, PartialEq, Clone)]
pub enum FilterOp {
    UserNameEq(String),
    DisplayNameEq(String),
    Invalid,
}

fn parse_filter_param(raw: &str) -> FilterOp {
    // RFC 7644 - 3.4.2.2.  Filtering
    //
    // Attribute names and attribute operators used in filters are case
    // insensitive.  For example, the following two expressions will
    // evaluate to the same logical value:
    //
    // filter=userName Eq "john"
    //
    // filter=Username eq "john"
    //
    // The filter parameter MUST contain at least one valid expression.  Each
    // expression MUST contain an attribute name followed by an attribute
    // operator and optional value.  Multiple expressions MAY be combined using
    // logical operators.  Expressions MAY be grouped together
    // using round brackets "(" and ")"
    //
    // NOTE: We are currently only supporting a single `Eq` operator but we
    // may expand to support more filtering operators and expressions in the
    // future. Additionally we are explicitly looking for "userName" and
    // "displayName" at this time. In the future we should consider using the
    // winnow crate to properly parse these operations.

    let raw = raw.trim().to_lowercase();
    let parts: Vec<_> = raw.split(" eq ").collect();

    if parts.len() != 2 {
        return FilterOp::Invalid;
    }

    // The value portion of the expression must be a string wrapped in
    // quotations
    let is_quoted_value = |v: &str| -> Option<String> {
        if v.len() > 2 && v.starts_with('"') && v.ends_with('"') {
            return Some(v[1..v.len() - 1].to_string());
        }
        None
    };

    match (parts[0], is_quoted_value(parts[1])) {
        ("username", Some(value)) => FilterOp::UserNameEq(value),
        ("displayname", Some(value)) => FilterOp::DisplayNameEq(value),
        _ => FilterOp::Invalid,
    }
}

#[cfg(test)]
mod test {
    use super::parse_filter_param;
    use crate::FilterOp;

    #[test]
    fn test_user_eq_filter() {
        assert_eq!(
            parse_filter_param("userName Eq \"Mike\""),
            FilterOp::UserNameEq("mike".to_string())
        );

        assert_eq!(
            parse_filter_param("USERNAME eq \"JAMES\""),
            FilterOp::UserNameEq("james".to_string())
        );

        assert_eq!(
            parse_filter_param(
                "USERNAME eq \"michael+dakota@oxidecomputer.com\""
            ),
            FilterOp::UserNameEq(
                "michael+dakota@oxidecomputer.com".to_string()
            )
        );
    }

    #[test]
    fn test_group_eq_filter() {
        assert_eq!(
            parse_filter_param("displayName Eq \"PowerUsers\""),
            FilterOp::DisplayNameEq("powerusers".to_string())
        );

        assert_eq!(
            parse_filter_param("dIsPlAyNaMe EQ \"Admins\""),
            FilterOp::DisplayNameEq("admins".to_string())
        );
    }

    #[test]
    fn test_invalid_filter() {
        assert_eq!(
            parse_filter_param("displayName Eq \"PowerUsers\" extra values"),
            FilterOp::Invalid
        );

        assert_eq!(
            parse_filter_param("extra value username EQ \"Admins\""),
            FilterOp::Invalid
        );

        assert_eq!(
            parse_filter_param("meta.lastModified gt \"2011-05-13T04:42:34Z\""),
            FilterOp::Invalid
        );
    }
}
