// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This crate implements a subset of System for Cross-domain Identity
//! Management version 2.0 (SCIM) or RFC 7643 (schema) and RFC 7644 (protocol).
//! At the moment it is known to work specifically with Okta serving as an IdP.

mod group;
mod in_memory_provider_store;
mod meta;
mod patch;
mod provider;
mod provider_store;
mod query_params;
mod resource;
mod response;
mod user;
mod utils;

pub use group::CreateGroupRequest;
pub use group::Group;
pub use group::GroupMember;
pub use in_memory_provider_store::InMemoryProviderStore;
pub use in_memory_provider_store::InMemoryProviderStoreState;
pub use meta::Meta;
pub use meta::StoredMeta;
pub use meta::StoredParts;
pub use patch::PatchRequest;
pub use patch::PatchRequestError;
pub use provider::Provider;
pub use provider_store::ProviderStore;
pub use provider_store::ProviderStoreDeleteResult;
pub use provider_store::ProviderStoreError;
pub use query_params::FilterOp;
pub use query_params::QueryParams;
pub use resource::Resource;
pub use response::Error;
pub use response::ErrorType;
pub use response::ListResponse;
pub use response::SingleResourceResponse;
pub use user::CreateUserRequest;
pub use user::User;
pub use user::UserGroup;
pub use user::UserGroupType;
pub use utils::ResourceType;
