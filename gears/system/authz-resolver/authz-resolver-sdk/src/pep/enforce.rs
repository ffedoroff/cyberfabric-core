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

#[cfg(test)]
mod tests {
    use super::*;

    const RES: ResourceType = ResourceType::from_static("gts.test.thing.v1~", &[]);

    /// Routes the two methods to distinguishable scopes (allow-all vs deny-all)
    /// so a test can assert which one was reached through a wrapper.
    struct RoutingMock;

    #[async_trait]
    impl Enforce for RoutingMock {
        async fn access_scope(
            &self,
            _ctx: &SecurityContext,
            _resource: &ResourceType,
            _action: &str,
            _resource_id: Option<Uuid>,
        ) -> Result<AccessScope, EnforcerError> {
            Ok(AccessScope::allow_all())
        }

        async fn access_scope_with(
            &self,
            _ctx: &SecurityContext,
            _resource: &ResourceType,
            _action: &str,
            _resource_id: Option<Uuid>,
            _request: &AccessRequest,
        ) -> Result<AccessScope, EnforcerError> {
            Ok(AccessScope::deny_all())
        }
    }

    #[tokio::test]
    async fn arc_dyn_forwards_access_scope() {
        let pep: Arc<dyn Enforce> = Arc::new(RoutingMock);
        let scope = pep
            .access_scope(&SecurityContext::anonymous(), &RES, "get", None)
            .await
            .expect("mock never errors");
        assert_eq!(scope, AccessScope::allow_all());
    }

    #[tokio::test]
    async fn arc_dyn_forwards_access_scope_with() {
        let pep: Arc<dyn Enforce> = Arc::new(RoutingMock);
        let scope = pep
            .access_scope_with(
                &SecurityContext::anonymous(),
                &RES,
                "list",
                None,
                &AccessRequest::new(),
            )
            .await
            .expect("mock never errors");
        // Distinct from `access_scope` — proves the right method is forwarded.
        assert_eq!(scope, AccessScope::deny_all());
    }

    #[tokio::test]
    async fn blanket_impl_covers_arc_of_concrete_type() {
        // `Arc<T>` (not only `Arc<dyn Enforce>`) implements `Enforce`, so it can
        // be handed to an `impl Enforce` parameter and called directly.
        let pep = Arc::new(RoutingMock);
        let scope = pep
            .access_scope(&SecurityContext::anonymous(), &RES, "get", None)
            .await
            .expect("mock never errors");
        assert_eq!(scope, AccessScope::allow_all());
    }
}
