//! Integration read service for external consumers (e.g., `AuthZ` plugin).
//!
//! Provides a thin adapter over `GroupService` implementing the SDK
//! `ResourceGroupReadHierarchy` and `ResourceGroupReadPluginClient` traits.

// @cpt-dod:cpt-cf-resource-group-dod-integration-auth-read-service:p1

use std::sync::Arc;

use async_trait::async_trait;
use modkit_odata::{ODataQuery, Page};
use modkit_security::SecurityContext;
use resource_group_sdk::error::ResourceGroupError;
use resource_group_sdk::models::{ResourceGroupMembership, ResourceGroupWithDepth};
use resource_group_sdk::{ResourceGroupReadHierarchy, ResourceGroupReadPluginClient};
use uuid::Uuid;

use crate::domain::group_service::GroupService;
use crate::domain::membership_service::MembershipService;

/// Adapter service exposing hierarchy reads via SDK traits.
///
/// Registered with `ClientHub` so that external consumers (`AuthZ` plugin)
/// can resolve `dyn ResourceGroupReadHierarchy` without depending on the
/// module's internal domain types.
#[allow(unknown_lints, de0309_must_have_domain_model)]
pub struct RgReadService {
    group_service: Arc<GroupService>,
    membership_service: Arc<MembershipService>,
}

impl RgReadService {
    /// Create a new `RgReadService`.
    #[must_use]
    pub fn new(
        group_service: Arc<GroupService>,
        membership_service: Arc<MembershipService>,
    ) -> Self {
        Self {
            group_service,
            membership_service,
        }
    }
}

#[async_trait]
impl ResourceGroupReadHierarchy for RgReadService {
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
}

#[async_trait]
impl ResourceGroupReadPluginClient for RgReadService {
    async fn list_memberships(
        &self,
        ctx: &SecurityContext,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        self.membership_service
            .list_memberships(ctx, query)
            .await
            .map_err(ResourceGroupError::from)
    }
}
