// @cpt-dod:cpt-cf-resource-group-dod-sdk-foundation-sdk-traits:p1
//! SDK trait contracts for the resource-group module.

use async_trait::async_trait;
use modkit_security::SecurityContext;

use modkit_odata::{ODataQuery, Page};
use uuid::Uuid;

use crate::error::ResourceGroupError;
use crate::models::{
    CreateTypeRequest, ResourceGroupMembership, ResourceGroupType, ResourceGroupWithDepth,
    UpdateTypeRequest,
};

/// Client trait for resource-group type management.
///
/// Consumers obtain this from `ClientHub`:
/// ```ignore
/// let client = hub.get::<dyn ResourceGroupClient>()?;
/// let rg_type = client.get_type(&ctx, "gts.x.system.rg.type.v1~...").await?;
/// ```
#[async_trait]
pub trait ResourceGroupClient: Send + Sync {
    // -- Type lifecycle --

    /// Create a new GTS type definition.
    async fn create_type(
        &self,
        ctx: &SecurityContext,
        request: CreateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError>;

    /// Get a GTS type definition by its code (GTS type path).
    async fn get_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
    ) -> Result<ResourceGroupType, ResourceGroupError>;

    /// List all GTS type definitions.
    async fn list_types(
        &self,
        ctx: &SecurityContext,
    ) -> Result<Vec<ResourceGroupType>, ResourceGroupError>;

    /// Update a GTS type definition (full replacement).
    async fn update_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
        request: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, ResourceGroupError>;

    /// Delete a GTS type definition. Fails if groups of this type exist.
    async fn delete_type(
        &self,
        ctx: &SecurityContext,
        code: &str,
    ) -> Result<(), ResourceGroupError>;
}

// @cpt-dod:cpt-cf-resource-group-dod-integration-auth-read-service:p1
/// Narrow read-only trait for hierarchy data, used by AuthZ plugin.
///
/// This trait provides the integration read port that external consumers
/// (such as the AuthZ plugin) use to query group hierarchy data without
/// depending on the full `ResourceGroupClient`.
#[async_trait]
pub trait ResourceGroupReadHierarchy: Send + Sync {
    /// List group hierarchy with depth for a given group.
    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError>;
}

// @cpt-flow:cpt-cf-resource-group-flow-integration-auth-plugin-routing:p1
/// Extended read trait for vendor-specific plugin gateway routing.
///
/// Extends `ResourceGroupReadHierarchy` with membership listing for
/// plugin consumers that need both hierarchy and membership data.
#[async_trait]
pub trait ResourceGroupReadPluginClient: ResourceGroupReadHierarchy {
    /// List memberships for plugin consumers.
    async fn list_memberships(
        &self,
        ctx: &SecurityContext,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError>;
}
