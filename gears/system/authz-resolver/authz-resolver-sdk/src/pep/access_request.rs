//! [`AccessRequest`] — per-request evaluation parameters for advanced
//! authorization scenarios.

use std::collections::HashMap;

use uuid::Uuid;

use crate::models::{BarrierMode, TenantContext, TenantMode};
use crate::property_value::IntoPropertyValue;

/// Per-request evaluation parameters for advanced authorization scenarios.
///
/// Used with [`PolicyEnforcer::access_scope_with()`](crate::pep::PolicyEnforcer::access_scope_with)
/// when the simple [`PolicyEnforcer::access_scope()`](crate::pep::PolicyEnforcer::access_scope)
/// defaults don't suffice (ABAC resource properties, custom tenant mode, barrier
/// bypass, etc.).
///
/// All fields default to "not overridden" - only set what you need.
///
/// # Examples
///
/// ```ignore
/// use authz_resolver_sdk::pep::{AccessRequest, PolicyEnforcer, ResourceType};
///
/// // CREATE with target tenant + resource properties (constrained scope)
/// let scope = enforcer.access_scope_with(
///     &ctx, &RESOURCE, "create", None,
///     &AccessRequest::new()
///         .context_tenant_id(target_tenant_id)
///         .tenant_mode(TenantMode::RootOnly)
///         .resource_property(pep_properties::OWNER_TENANT_ID, target_tenant_id),
/// ).await?;
///
/// // Billing - ignore barriers (constrained scope)
/// let scope = enforcer.access_scope_with(
///     &ctx, &RESOURCE, "list", None,
///     &AccessRequest::new().barrier_mode(BarrierMode::Ignore),
/// ).await?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct AccessRequest {
    pub(crate) resource_properties: HashMap<String, serde_json::Value>,
    pub(crate) tenant_context: Option<TenantContext>,
    pub(crate) require_constraints: Option<bool>,
}

impl AccessRequest {
    /// Create a new empty access request (all defaults).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single resource property for ABAC evaluation.
    #[must_use]
    pub fn resource_property(
        mut self,
        key: impl Into<String>,
        value: impl IntoPropertyValue,
    ) -> Self {
        self.resource_properties
            .insert(key.into(), value.into_filter_value());
        self
    }

    /// Set all resource properties at once (replaces any previously set).
    #[must_use]
    pub fn resource_properties(mut self, props: HashMap<String, serde_json::Value>) -> Self {
        self.resource_properties = props;
        self
    }

    /// Override the context tenant ID (default: subject's tenant).
    #[must_use]
    pub fn context_tenant_id(mut self, id: Uuid) -> Self {
        self.tenant_context.get_or_insert_default().root_id = Some(id);
        self
    }

    /// Override the tenant hierarchy mode (default: `Subtree`).
    #[must_use]
    pub fn tenant_mode(mut self, mode: TenantMode) -> Self {
        self.tenant_context.get_or_insert_default().mode = mode;
        self
    }

    /// Override the barrier enforcement mode (default: `Respect`).
    #[must_use]
    pub fn barrier_mode(mut self, mode: BarrierMode) -> Self {
        self.tenant_context.get_or_insert_default().barrier_mode = mode;
        self
    }

    /// Set a tenant status filter (e.g., `["active"]`).
    #[must_use]
    pub fn tenant_status(mut self, statuses: Vec<String>) -> Self {
        self.tenant_context.get_or_insert_default().tenant_status = Some(statuses);
        self
    }

    /// Set the entire tenant context at once.
    #[must_use]
    pub fn tenant_context(mut self, tc: TenantContext) -> Self {
        self.tenant_context = Some(tc);
        self
    }

    /// Override the `require_constraints` flag (default: `true`).
    ///
    /// When `false`, the PDP is told that constraints are optional.
    /// If the PDP returns no constraints, the resulting scope is
    /// `allow_all()` (no row-level filtering). If the PDP still returns
    /// constraints, they are compiled normally.
    ///
    /// Primary use cases:
    /// - **GET with prefetch**: if scope is unconstrained, return the
    ///   prefetched entity directly; otherwise do a scoped re-read.
    /// - **CREATE**: if scope is unconstrained, skip insert validation;
    ///   otherwise validate the insert against the scope.
    #[must_use]
    pub fn require_constraints(mut self, require: bool) -> Self {
        self.require_constraints = Some(require);
        self
    }
}
