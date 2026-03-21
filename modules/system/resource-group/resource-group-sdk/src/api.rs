//! SDK trait contracts for the resource-group module.

use async_trait::async_trait;
use modkit_security::SecurityContext;

use crate::error::ResourceGroupError;
use crate::models::{CreateTypeRequest, ResourceGroupType, UpdateTypeRequest};

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
