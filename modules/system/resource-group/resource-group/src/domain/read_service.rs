//! Integration read service for external consumers (e.g., AuthZ plugin).
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

/// Adapter service exposing hierarchy reads via SDK traits.
///
/// Registered with `ClientHub` so that external consumers (AuthZ plugin)
/// can resolve `dyn ResourceGroupReadHierarchy` without depending on the
/// module's internal domain types.
pub struct RgReadService {
    group_service: Arc<GroupService>,
}

impl RgReadService {
    /// Create a new `RgReadService`.
    #[must_use]
    pub fn new(group_service: Arc<GroupService>) -> Self {
        Self { group_service }
    }
}

#[async_trait]
impl ResourceGroupReadHierarchy for RgReadService {
    async fn list_group_depth(
        &self,
        _ctx: &SecurityContext,
        group_id: Uuid,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
        self.group_service
            .list_group_hierarchy(group_id, query)
            .await
            .map_err(ResourceGroupError::from)
    }
}

#[async_trait]
impl ResourceGroupReadPluginClient for RgReadService {
    async fn list_memberships(
        &self,
        _ctx: &SecurityContext,
        _query: &ODataQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError> {
        // Membership service is not yet implemented (Feature 4: Membership).
        // Return an empty page until the membership domain is available.
        Ok(Page {
            items: Vec::new(),
            page_info: modkit_odata::PageInfo {
                next_cursor: None,
                prev_cursor: None,
                limit: _query.limit.unwrap_or(20),
            },
        })
    }
}
