// @cpt-req:cpt-cf-resource-group-dod-sdk-crate:p1

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

pub mod api;
pub mod error;
pub mod models;

pub use api::{ResourceGroupClient, ResourceGroupReadHierarchy};
pub use error::ResourceGroupError;
pub use models::{
    AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery, Page, PageInfo,
    RemoveMembershipRequest, ResourceGroup, ResourceGroupMembership, ResourceGroupType,
    ResourceGroupWithDepth, UpdateGroupRequest, UpdateTypeRequest,
};
