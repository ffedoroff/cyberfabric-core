//! Unified service adapter implementing `ResourceGroupClient` for `ClientHub` registration.
//!
//! Delegates to `TypeService`, `GroupService`, and `MembershipService` to satisfy
//! the full SDK trait contract.

use std::sync::Arc;

use async_trait::async_trait;
use modkit_odata::{ODataQuery, Page};
use modkit_security::SecurityContext;
use resource_group_sdk::ResourceGroupClient;
use resource_group_sdk::error::ResourceGroupError;
use resource_group_sdk::models::{
    CreateGroupRequest, CreateTypeRequest, ResourceGroup, ResourceGroupMembership,
    ResourceGroupType, ResourceGroupWithDepth, UpdateGroupRequest, UpdateTypeRequest,
};
use uuid::Uuid;

use crate::domain::group_service::GroupService;
use crate::domain::membership_service::MembershipService;
use crate::domain::type_service::TypeService;

/// Unified adapter registered with `ClientHub` as `dyn ResourceGroupClient`.
#[allow(unknown_lints, de0309_must_have_domain_model)]
#[allow(clippy::struct_field_names)]
pub struct RgService {
    type_service: Arc<TypeService>,
    group_service: Arc<GroupService>,
    membership_service: Arc<MembershipService>,
}

impl RgService {
    /// Create a new `RgService`.
    #[must_use]
    pub fn new(
        type_service: Arc<TypeService>,
        group_service: Arc<GroupService>,
        membership_service: Arc<MembershipService>,
    ) -> Self {
        Self {
            type_service,
            group_service,
            membership_service,
        }
    }
}

#[async_trait]
impl ResourceGroupClient for RgService {
    // -- Type lifecycle --

    async fn create_type(
        &self,
        _ctx: &SecurityContext,
        request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        self.type_service
            .create_type(request)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn get_type(
        &self,
        _ctx: &SecurityContext,
        code: &str,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        self.type_service
            .get_type(code)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn list_types(
        &self,
        _ctx: &SecurityContext,
    ) -> Result<Vec<ResourceGroupType>, ResourceGroupError> {
        self.type_service
            .list_types()
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn update_type(
        &self,
        _ctx: &SecurityContext,
        code: &str,
        request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError> {
        self.type_service
            .update_type(code, request)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn delete_type(
        &self,
        _ctx: &SecurityContext,
        code: &str,
    ) -> Result<(), ResourceGroupError> {
        self.type_service
            .delete_type(code)
            .await
            .map_err(ResourceGroupError::from)
    }

    // -- Group lifecycle --

    async fn create_group(
        &self,
        ctx: &SecurityContext,
        request: CreateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        let tenant_id = ctx.subject_tenant_id();
        self.group_service
            .create_group(request, tenant_id)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn get_group(
        &self,
        ctx: &SecurityContext,
        id: Uuid,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        self.group_service
            .get_group(ctx, id)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn list_groups(
        &self,
        ctx: &SecurityContext,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroup>, ResourceGroupError> {
        self.group_service
            .list_groups(ctx, query)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn update_group(
        &self,
        ctx: &SecurityContext,
        id: Uuid,
        request: UpdateGroupRequest,
    ) -> Result<ResourceGroup, ResourceGroupError> {
        self.group_service
            .update_group(ctx, id, request)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn delete_group(
        &self,
        ctx: &SecurityContext,
        id: Uuid,
    ) -> Result<(), ResourceGroupError> {
        self.group_service
            .delete_group(ctx, id, false)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        self.group_service
            .list_group_hierarchy(ctx, group_id, query)
            .await
            .map_err(ResourceGroupError::from)
    }

    // -- Membership lifecycle --

    async fn add_membership(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<ResourceGroupMembership, ResourceGroupError> {
        self.membership_service
            .add_membership(group_id, resource_type, resource_id)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn remove_membership(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), ResourceGroupError> {
        self.membership_service
            .remove_membership(group_id, resource_type, resource_id)
            .await
            .map_err(ResourceGroupError::from)
    }

    async fn list_memberships(
        &self,
        _ctx: &SecurityContext,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        self.membership_service
            .list_memberships(query)
            .await
            .map_err(ResourceGroupError::from)
    }
}
