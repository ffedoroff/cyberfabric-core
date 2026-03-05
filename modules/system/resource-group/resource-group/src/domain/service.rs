use std::sync::Arc;

use async_trait::async_trait;
use modkit_security::SecurityContext;
use resource_group_sdk::{
    AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery, Page,
    RemoveMembershipRequest, ResourceGroup, ResourceGroupClient, ResourceGroupError,
    ResourceGroupMembership, ResourceGroupReadHierarchy, ResourceGroupType, ResourceGroupWithDepth,
    UpdateGroupRequest, UpdateTypeRequest,
};
use uuid::Uuid;

/// Unified service facade implementing both SDK traits.
/// Backed by domain services and repositories.
/// Domain logic (type validation, hierarchy invariants, membership rules)
/// will be implemented in Features 2-5.
pub struct RgService {
    _db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
}

impl RgService {
    #[must_use]
    pub fn new(db: Arc<modkit_db::DBProvider<modkit_db::DbError>>) -> Self {
        Self { _db: db }
    }
}

// Stub implementations — domain logic is out of scope for Feature 0001.
// Each method returns a placeholder error indicating the feature is not yet implemented.

fn not_implemented() -> ResourceGroupError {
    ResourceGroupError::Internal
}

#[async_trait]
impl ResourceGroupClient for RgService {
    async fn create_type(
        &self,
        _ctx: &SecurityContext,
        _request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn get_type(
        &self,
        _ctx: &SecurityContext,
        _code: &str,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_types(
        &self,
        _ctx: &SecurityContext,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroupType>, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn update_type(
        &self,
        _ctx: &SecurityContext,
        _code: &str,
        _request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn delete_type(
        &self,
        _ctx: &SecurityContext,
        _code: &str,
    ) -> Result<(), ResourceGroupError> {
        Err(not_implemented())
    }

    async fn create_group(
        &self,
        _ctx: &SecurityContext,
        _request: CreateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn get_group(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_groups(
        &self,
        _ctx: &SecurityContext,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroup>, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn update_group(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
        _request: UpdateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn delete_group(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
        _force: bool,
    ) -> Result<(), ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_group_depth(
        &self,
        _ctx: &SecurityContext,
        _group_id: Uuid,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn add_membership(
        &self,
        _ctx: &SecurityContext,
        _request: AddMembershipRequest,
    ) -> Result<ResourceGroupMembership, ResourceGroupError> {
        Err(not_implemented())
    }

    async fn remove_membership(
        &self,
        _ctx: &SecurityContext,
        _request: RemoveMembershipRequest,
    ) -> Result<(), ResourceGroupError> {
        Err(not_implemented())
    }

    async fn list_memberships(
        &self,
        _ctx: &SecurityContext,
        _query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        Err(not_implemented())
    }
}

#[async_trait]
impl ResourceGroupReadHierarchy for RgService {
    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        ResourceGroupClient::list_group_depth(self, ctx, group_id, query).await
    }
}
