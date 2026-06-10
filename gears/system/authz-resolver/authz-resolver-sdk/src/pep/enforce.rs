//! [`Enforce`] — consumer-facing PEP trait (dependency inversion).
//!
//! Services depend on this narrow trait (`Arc<dyn Enforce>` or a generic bound)
//! rather than the concrete [`PolicyEnforcer`](crate::pep::PolicyEnforcer), so a
//! change to the enforcer implementation does not ripple into every call site.
//! The concrete `PolicyEnforcer` is constructed once (in a gear's wiring) and
//! handed to services as `Arc<dyn Enforce>`.

use std::sync::Arc;

use async_trait::async_trait;
use toolkit_security::{AccessScope, SecurityContext};
use uuid::Uuid;

use crate::pep::access_request::AccessRequest;
use crate::pep::enforcer_error::EnforcerError;
use crate::pep::resource_type::ResourceType;

/// The PEP authorization entry point used by services.
///
/// Implemented by [`PolicyEnforcer`](crate::pep::PolicyEnforcer); the two
/// methods mirror its inherent flow methods.
#[async_trait]
pub trait Enforce: Send + Sync {
    /// Execute the full PEP flow with constraints: build request → evaluate
    /// → compile constraints to `AccessScope`.
    ///
    /// # Errors
    ///
    /// - [`EnforcerError::EvaluationFailed`] if the PDP call fails
    /// - [`EnforcerError::CompileFailed`] if constraint compilation fails
    async fn access_scope(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
    ) -> Result<AccessScope, EnforcerError>;

    /// Execute the full PEP flow with constraints and per-request overrides.
    ///
    /// # Errors
    ///
    /// - [`EnforcerError::EvaluationFailed`] if the PDP call fails
    /// - [`EnforcerError::CompileFailed`] if constraint compilation fails
    async fn access_scope_with(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
        request: &AccessRequest,
    ) -> Result<AccessScope, EnforcerError>;
}

/// Blanket impl so smart-pointer wrappers (`Arc<dyn Enforce>`,
/// `Arc<PolicyEnforcer>`) can be passed wherever `impl Enforce` is expected and
/// handed onward between services without unwrapping.
#[async_trait]
impl<T: Enforce + ?Sized> Enforce for Arc<T> {
    async fn access_scope(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
    ) -> Result<AccessScope, EnforcerError> {
        (**self).access_scope(ctx, resource, action, resource_id).await
    }

    async fn access_scope_with(
        &self,
        ctx: &SecurityContext,
        resource: &ResourceType,
        action: &str,
        resource_id: Option<Uuid>,
        request: &AccessRequest,
    ) -> Result<AccessScope, EnforcerError> {
        (**self)
            .access_scope_with(ctx, resource, action, resource_id, request)
            .await
    }
}
