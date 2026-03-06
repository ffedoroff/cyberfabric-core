// @cpt-req:cpt-cf-resource-group-dod-sdk-crate:p1
// @cpt-flow:cpt-cf-resource-group-flow-sdk-client-resolution:p2

use async_trait::async_trait;
use modkit_security::SecurityContext;
use uuid::Uuid;

use crate::error::ResourceGroupError;
use crate::models::{
    AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery, Page,
    RemoveMembershipRequest, ResourceGroup, ResourceGroupMembership, ResourceGroupType,
    ResourceGroupWithDepth, UpdateGroupRequest, UpdateTypeRequest,
};

/// Full read+write client for Resource Group operations.
/// Registered in `ClientHub` as `Arc<dyn ResourceGroupClient>`.
/// Used by domain clients and general consumers.
#[async_trait]
pub trait ResourceGroupClient: Send + Sync {
    // ── Type lifecycle ──────────────────────────────────────────────
    async fn create_type(
        &self,
        ctx: &SecurityContext,
        request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError>;

    async fn get_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
    ) -> Result<ResourceGroupType, ResourceGroupError>;

    async fn list_types(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupType>, ResourceGroupError>;

    async fn update_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
        request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError>;

    async fn delete_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
    ) -> Result<(), ResourceGroupError>;

    // ── Group lifecycle ─────────────────────────────────────────────
    async fn create_group(
        &self,
        ctx: &SecurityContext,
        request: CreateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError>;

    async fn get_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
    ) -> Result<ResourceGroup, ResourceGroupError>;

    async fn list_groups(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroup>, ResourceGroupError>;

    async fn update_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        request: UpdateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError>;

    async fn delete_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        force: bool,
    ) -> Result<(), ResourceGroupError>;

    // ── Hierarchy ───────────────────────────────────────────────────
    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError>;

    // ── Membership lifecycle ────────────────────────────────────────
    async fn add_membership(
        &self,
        ctx: &SecurityContext,
        request: AddMembershipRequest,
    ) -> Result<ResourceGroupMembership, ResourceGroupError>;

    async fn remove_membership(
        &self,
        ctx: &SecurityContext,
        request: RemoveMembershipRequest,
    ) -> Result<(), ResourceGroupError>;

    async fn list_memberships(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError>;
}

/// Narrow hierarchy-only read contract.
/// Used by `AuthZ` plugin — provides only hierarchy traversal, no memberships.
/// Registered in `ClientHub` as `Arc<dyn ResourceGroupReadHierarchy>`.
#[async_trait]
pub trait ResourceGroupReadHierarchy: Send + Sync {
    /// Matches REST `GET /groups/{group_id}/depth` with `OData` query.
    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError>;
}
