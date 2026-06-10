// Updated: 2026-04-14 by Constructor Tech
//! Policy Enforcement Point (`PEP`) object.
//!
//! [`PolicyEnforcer`] encapsulates the full PEP flow:
//! build evaluation request → call PDP → compile constraints to `AccessScope`.
//!
//! Constructed once during service initialisation with the `AuthZ` client.
//! The resource type is supplied per call via a [`ResourceType`] descriptor,
//! so a single enforcer can serve all resource types in a service.
//!
//! Services should depend on the [`Enforce`] trait (`Arc<dyn Enforce>`) rather
//! than this concrete type; see [`crate::pep::enforce`].

use std::sync::Arc;

use async_trait::async_trait;
use toolkit_security::{AccessScope, SecurityContext};
use uuid::Uuid;

use crate::api::AuthZResolverClient;
use crate::models::{Capability, EvaluationRequest};
use crate::pep::access_request::AccessRequest;
use crate::pep::compiler::compile_to_access_scope;
use crate::pep::enforce::Enforce;
use crate::pep::enforcer_error::EnforcerError;
use crate::pep::request_builder;
use crate::pep::resource_type::ResourceType;

/// Policy Enforcement Point.
///
/// Holds the `AuthZ` client and optional PEP capabilities.
/// Constructed once during service init; cloneable and cheap to pass
/// around (`Arc` inside). The resource type is supplied per call via
/// [`ResourceType`].
///
/// # Example
///
/// ```ignore
/// use authz_resolver_sdk::pep::{PolicyEnforcer, ResourceType};
/// use toolkit_security::pep_properties;
///
/// const USER: ResourceType = ResourceType::from_static(
///     "gts.cf.core.users.user.v1~",
///     &[pep_properties::OWNER_TENANT_ID, pep_properties::RESOURCE_ID],
/// );
///
/// let enforcer = PolicyEnforcer::new(authz.clone());
///
/// // All CRUD operations return AccessScope (PDP always returns constraints)
/// let scope = enforcer.access_scope(&ctx, &USER, "get", Some(id)).await?;
/// let scope = enforcer.access_scope(&ctx, &USER, "create", None).await?;
/// ```
#[derive(Clone)]
pub struct PolicyEnforcer {
    authz: Arc<dyn AuthZResolverClient>,
    capabilities: Vec<Capability>,
}

impl PolicyEnforcer {
    /// Create a new enforcer.
    pub fn new(authz: Arc<dyn AuthZResolverClient>) -> Self {
        Self {
            authz,
            capabilities: Vec::new(),
        }
    }

    /// Set PEP capabilities advertised to the PDP.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    // ── Low-level: build request only ────────────────────────────────

    /// Build an evaluation request using the subject's tenant as context tenant
    /// and default settings.
    #[must_use]
    pub fn build_request(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
        require_constraints: bool,
    ) -> EvaluationRequest {
        request_builder::build_request_with(
            &self.capabilities,
            ctx,
            resource,
            action,
            resource_id,
            require_constraints,
            &AccessRequest::default(),
        )
    }

    /// Build an evaluation request with per-request overrides from [`AccessRequest`].
    #[must_use]
    pub fn build_request_with(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
        require_constraints: bool,
        request: &AccessRequest,
    ) -> EvaluationRequest {
        request_builder::build_request_with(
            &self.capabilities,
            ctx,
            resource,
            action,
            resource_id,
            require_constraints,
            request,
        )
    }

    // ── High-level: full PEP flow (all CRUD operations) ─────────────

    /// Execute the full PEP flow with constraints: build request → evaluate
    /// → compile constraints to `AccessScope`.
    ///
    /// Always sets `require_constraints=true`. PDP returns constraints for
    /// all CRUD operations (GET, LIST, UPDATE, DELETE, CREATE).
    ///
    /// # Errors
    ///
    /// - [`EnforcerError::EvaluationFailed`] if the PDP call fails
    /// - [`EnforcerError::CompileFailed`] if constraint compilation fails (denied, missing, etc.)
    pub async fn access_scope(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
    ) -> Result<AccessScope, EnforcerError> {
        self.access_scope_with(ctx, resource, action, resource_id, &AccessRequest::default())
            .await
    }

    /// Execute the full PEP flow with constraints and per-request overrides.
    ///
    /// Uses `require_constraints` from [`AccessRequest`] (default: `true`).
    /// When `false`, the PDP may return no constraints; the resulting scope
    /// is `allow_all()`. When `true`, empty constraints trigger a compile error.
    ///
    /// # Errors
    ///
    /// - [`EnforcerError::EvaluationFailed`] if the PDP call fails
    /// - [`EnforcerError::CompileFailed`] if constraint compilation fails (denied, missing, etc.)
    pub async fn access_scope_with(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
        request: &AccessRequest,
    ) -> Result<AccessScope, EnforcerError> {
        let require = request.require_constraints.unwrap_or(true);
        let eval_request =
            self.build_request_with(ctx, resource, action, resource_id, require, request);
        let response = self.authz.evaluate(eval_request).await?;

        // Check decision first: if denied, return error immediately
        // without attempting constraint compilation.
        if !response.decision {
            return Err(EnforcerError::Denied {
                deny_reason: response.context.deny_reason,
            });
        }

        Ok(compile_to_access_scope(
            &response,
            require,
            resource.supported_properties(),
        )?)
    }
}

#[async_trait]
impl Enforce for PolicyEnforcer {
    async fn access_scope(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
    ) -> Result<AccessScope, EnforcerError> {
        PolicyEnforcer::access_scope(self, ctx, resource, action, resource_id).await
    }

    async fn access_scope_with(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
        request: &AccessRequest,
    ) -> Result<AccessScope, EnforcerError> {
        PolicyEnforcer::access_scope_with(self, ctx, resource, action, resource_id, request).await
    }
}

impl std::fmt::Debug for PolicyEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyEnforcer")
            .field("capabilities", &self.capabilities)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "enforcer_tests.rs"]
mod enforcer_tests;
