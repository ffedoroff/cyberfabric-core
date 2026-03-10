//! Service implementation for the static `AuthZ` resolver plugin.

use std::sync::Arc;

use authz_resolver_sdk::{
    Capability, Constraint, EvaluationRequest, EvaluationResponse, EvaluationResponseContext,
    InPredicate, InTenantSubtreePredicate, Predicate, PredicateBarrierMode,
};
use modkit_macros::domain_model;
use modkit_security::{SecurityContext, pep_properties};
use resource_group_sdk::ResourceGroupReadHierarchy;
use uuid::Uuid;

/// Static `AuthZ` resolver service.
///
/// - Returns `decision: true` with an `in` predicate on `pep_properties::OWNER_TENANT_ID`
///   scoped to the context tenant from the request (for all operations including CREATE).
/// - When `GroupHierarchy` capability is declared, validates group ownership via
///   `ResourceGroupReadHierarchy` and includes descendant group tenant IDs in constraints.
/// - Denies access (`decision: false`) when no valid tenant can be resolved.
#[domain_model]
pub struct Service {
    hierarchy: Option<Arc<dyn ResourceGroupReadHierarchy>>,
}

impl Default for Service {
    fn default() -> Self {
        Self { hierarchy: None }
    }
}

impl Service {
    /// Create a service without hierarchy (for tests).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a service with hierarchy client (production init).
    #[must_use]
    pub fn with_hierarchy(hierarchy: Arc<dyn ResourceGroupReadHierarchy>) -> Self {
        Self {
            hierarchy: Some(hierarchy),
        }
    }

    /// Evaluate an authorization request.
    pub async fn evaluate(&self, request: &EvaluationRequest) -> EvaluationResponse {
        // Always scope to context tenant (all CRUD operations get constraints)
        let tenant_id = request
            .context
            .tenant_context
            .as_ref()
            .and_then(|t| t.root_id)
            .or_else(|| {
                // Fallback: extract tenant_id from subject properties
                request
                    .subject
                    .properties
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
            });

        let Some(tid) = tenant_id else {
            // No tenant resolvable from context or subject — deny access.
            return EvaluationResponse {
                decision: false,
                context: EvaluationResponseContext::default(),
            };
        };

        if tid == Uuid::default() {
            // Nil UUID tenant — deny rather than grant unrestricted access.
            return EvaluationResponse {
                decision: false,
                context: EvaluationResponseContext::default(),
            };
        }

        // Check if group hierarchy is requested and available
        let has_group_hierarchy = request
            .context
            .capabilities
            .contains(&Capability::GroupHierarchy);

        if has_group_hierarchy {
            if let Some(ref hierarchy) = self.hierarchy {
                if let Some(group_id) = Self::extract_group_id(request) {
                    return self
                        .evaluate_with_hierarchy(hierarchy, request, tid, group_id)
                        .await;
                }
            }
        }

        // Check if tenant hierarchy capability is declared — return InTenantSubtree
        let has_tenant_hierarchy = request
            .context
            .capabilities
            .contains(&Capability::TenantHierarchy);

        if has_tenant_hierarchy {
            return Self::allow_with_tenant_subtree(tid, request);
        }

        // Default: flat tenant-scoped constraint
        Self::allow_with_tenant(tid)
    }

    /// Evaluate with group hierarchy — verify group belongs to tenant, expand to subtree.
    async fn evaluate_with_hierarchy(
        &self,
        hierarchy: &Arc<dyn ResourceGroupReadHierarchy>,
        request: &EvaluationRequest,
        tenant_id: Uuid,
        group_id: Uuid,
    ) -> EvaluationResponse {
        let ctx = match Self::build_security_context(request, tenant_id) {
            Some(ctx) => ctx,
            None => return Self::allow_with_tenant(tenant_id),
        };

        // Query group subtree via ResourceGroupReadHierarchy
        match hierarchy
            .list_group_depth(&ctx, group_id, resource_group_sdk::ListQuery::default())
            .await
        {
            Ok(page) => {
                // Collect all tenant IDs from the hierarchy (the root group + descendants)
                let mut tenant_ids: Vec<Uuid> = page
                    .items
                    .iter()
                    .map(|g| g.tenant_id)
                    .filter(|t| *t == tenant_id)
                    .collect();

                if tenant_ids.is_empty() {
                    // Group doesn't belong to this tenant — deny
                    tracing::warn!(
                        group_id = %group_id,
                        tenant_id = %tenant_id,
                        "Group hierarchy check: group does not belong to tenant"
                    );
                    return EvaluationResponse {
                        decision: false,
                        context: EvaluationResponseContext::default(),
                    };
                }

                tenant_ids.dedup();

                Self::allow_with_tenant(tenant_id)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    group_id = %group_id,
                    "Group hierarchy lookup failed, falling back to tenant-only scope"
                );
                // Graceful degradation: fall back to tenant-scoped constraint
                Self::allow_with_tenant(tenant_id)
            }
        }
    }

    /// Extract group_id from resource properties or resource ID.
    fn extract_group_id(request: &EvaluationRequest) -> Option<Uuid> {
        // Try resource.properties["group_id"] first
        request
            .resource
            .properties
            .get("group_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            // Fall back to resource.id if resource_type looks like a group
            .or_else(|| {
                if request.resource.resource_type.contains("resource_group") {
                    request.resource.id
                } else {
                    None
                }
            })
    }

    /// Build a SecurityContext from an EvaluationRequest for internal RG queries.
    fn build_security_context(
        request: &EvaluationRequest,
        tenant_id: Uuid,
    ) -> Option<SecurityContext> {
        SecurityContext::builder()
            .subject_id(request.subject.id)
            .subject_tenant_id(tenant_id)
            .token_scopes(request.context.token_scopes.clone())
            .build()
            .ok()
    }

    /// Advanced allow response with tenant subtree predicate.
    ///
    /// Returned when `TenantHierarchy` capability is declared. Uses `InTenantSubtree`
    /// instead of flat `In` so the PEP can query the `tenant_closure` table.
    fn allow_with_tenant_subtree(
        tenant_id: Uuid,
        request: &EvaluationRequest,
    ) -> EvaluationResponse {
        let barrier_mode = request
            .context
            .tenant_context
            .as_ref()
            .map(|tc| match tc.barrier_mode {
                authz_resolver_sdk::BarrierMode::Respect => PredicateBarrierMode::Respect,
                authz_resolver_sdk::BarrierMode::Ignore => PredicateBarrierMode::Ignore,
            })
            .unwrap_or(PredicateBarrierMode::Respect);

        let tenant_status = request
            .context
            .tenant_context
            .as_ref()
            .and_then(|tc| tc.tenant_status.clone());

        let mut pred =
            InTenantSubtreePredicate::new(pep_properties::OWNER_TENANT_ID, tenant_id)
                .barrier_mode(barrier_mode);
        if let Some(statuses) = tenant_status {
            pred = pred.tenant_status(statuses);
        }

        EvaluationResponse {
            decision: true,
            context: EvaluationResponseContext {
                constraints: vec![Constraint {
                    predicates: vec![Predicate::InTenantSubtree(pred)],
                }],
                ..Default::default()
            },
        }
    }

    /// Standard allow response with tenant-scoped constraint.
    fn allow_with_tenant(tenant_id: Uuid) -> EvaluationResponse {
        EvaluationResponse {
            decision: true,
            context: EvaluationResponseContext {
                constraints: vec![Constraint {
                    predicates: vec![Predicate::In(InPredicate::new(
                        pep_properties::OWNER_TENANT_ID,
                        [tenant_id],
                    ))],
                }],
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use authz_resolver_sdk::pep::IntoPropertyValue;
    use authz_resolver_sdk::{Action, EvaluationRequestContext, Resource, Subject, TenantContext};
    use std::collections::HashMap;

    fn make_request(require_constraints: bool, tenant_id: Option<Uuid>) -> EvaluationRequest {
        let mut subject_properties = HashMap::new();
        subject_properties.insert(
            "tenant_id".to_owned(),
            serde_json::Value::String("22222222-2222-2222-2222-222222222222".to_owned()),
        );

        EvaluationRequest {
            subject: Subject {
                id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                subject_type: None,
                properties: subject_properties,
            },
            action: Action {
                name: "list".to_owned(),
            },
            resource: Resource {
                resource_type: "gts.x.core.users.user.v1~".to_owned(),
                id: None,
                properties: HashMap::new(),
            },
            context: EvaluationRequestContext {
                tenant_context: tenant_id.map(|id| TenantContext {
                    root_id: Some(id),
                    ..TenantContext::default()
                }),
                token_scopes: vec!["*".to_owned()],
                require_constraints,
                capabilities: vec![],
                supported_properties: vec![],
                bearer_token: None,
            },
        }
    }

    #[tokio::test]
    async fn list_operation_with_tenant_context() {
        let tenant_id = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
        let service = Service::new();
        let response = service.evaluate(&make_request(true, Some(tenant_id))).await;

        assert!(response.decision);
        assert_eq!(response.context.constraints.len(), 1);

        let constraint = &response.context.constraints[0];
        assert_eq!(constraint.predicates.len(), 1);

        match &constraint.predicates[0] {
            Predicate::In(in_pred) => {
                assert_eq!(in_pred.property, pep_properties::OWNER_TENANT_ID);
                assert_eq!(in_pred.values, vec![tenant_id.into_filter_value()]);
            }
            other => panic!("Expected In predicate, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_operation_without_tenant_falls_back_to_subject_properties() {
        let service = Service::new();
        let response = service.evaluate(&make_request(true, None)).await;

        // Falls back to subject.properties["tenant_id"]
        assert!(response.decision);
        assert_eq!(response.context.constraints.len(), 1);

        match &response.context.constraints[0].predicates[0] {
            Predicate::In(in_pred) => {
                assert_eq!(
                    in_pred.values,
                    vec![
                        Uuid::parse_str("22222222-2222-2222-2222-222222222222")
                            .unwrap()
                            .into_filter_value()
                    ]
                );
            }
            other => panic!("Expected In predicate, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn nil_tenant_is_denied() {
        let service = Service::new();
        let response = service
            .evaluate(&make_request(true, Some(Uuid::default())))
            .await;

        assert!(!response.decision);
        assert!(response.context.constraints.is_empty());
    }

    #[tokio::test]
    async fn missing_tenant_context_and_subject_property_is_denied() {
        let request = EvaluationRequest {
            subject: Subject {
                id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                subject_type: None,
                properties: HashMap::new(), // no tenant_id property
            },
            action: Action {
                name: "list".to_owned(),
            },
            resource: Resource {
                resource_type: "gts.x.core.users.user.v1~".to_owned(),
                id: None,
                properties: HashMap::new(),
            },
            context: EvaluationRequestContext {
                tenant_context: None,
                token_scopes: vec!["*".to_owned()],
                require_constraints: true,
                capabilities: vec![],
                supported_properties: vec![],
                bearer_token: None,
            },
        };

        let service = Service::new();
        let response = service.evaluate(&request).await;

        assert!(!response.decision);
        assert!(response.context.constraints.is_empty());
    }

    // ── TenantHierarchy capability tests ────────────────────────────────────

    #[tokio::test]
    async fn returns_in_tenant_subtree_when_tenant_hierarchy_capability() {
        let tenant_id = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
        let service = Service::new();

        let mut request = make_request(true, Some(tenant_id));
        request.context.capabilities = vec![Capability::TenantHierarchy];

        let response = service.evaluate(&request).await;

        assert!(response.decision);
        assert_eq!(response.context.constraints.len(), 1);

        match &response.context.constraints[0].predicates[0] {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(p.root_tenant_id, tenant_id);
                assert_eq!(p.property, pep_properties::OWNER_TENANT_ID);
                assert_eq!(
                    p.barrier_mode,
                    authz_resolver_sdk::PredicateBarrierMode::Respect
                );
            }
            other => panic!("Expected InTenantSubtree, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn returns_flat_in_without_tenant_hierarchy_capability() {
        let tenant_id = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
        let service = Service::new();
        let response = service.evaluate(&make_request(true, Some(tenant_id))).await;

        assert!(response.decision);
        match &response.context.constraints[0].predicates[0] {
            Predicate::In(p) => {
                assert_eq!(p.property, pep_properties::OWNER_TENANT_ID);
            }
            other => panic!("Expected flat In predicate, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn tenant_subtree_respects_barrier_mode_from_request() {
        let tenant_id = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
        let service = Service::new();

        let mut request = make_request(true, Some(tenant_id));
        request.context.capabilities = vec![Capability::TenantHierarchy];
        request.context.tenant_context.as_mut().unwrap().barrier_mode =
            authz_resolver_sdk::BarrierMode::Ignore;
        request.context.tenant_context.as_mut().unwrap().tenant_status =
            Some(vec!["active".to_owned()]);

        let response = service.evaluate(&request).await;

        match &response.context.constraints[0].predicates[0] {
            Predicate::InTenantSubtree(p) => {
                assert_eq!(
                    p.barrier_mode,
                    authz_resolver_sdk::PredicateBarrierMode::Ignore
                );
                assert_eq!(
                    p.tenant_status,
                    Some(vec!["active".to_owned()])
                );
            }
            other => panic!("Expected InTenantSubtree, got: {other:?}"),
        }
    }

    // ── Group hierarchy tests ──────────────────────────────────────────────

    mod hierarchy {
        use super::*;
        use async_trait::async_trait;
        use resource_group_sdk::{
            ListQuery, Page, PageInfo, ResourceGroupError, ResourceGroupWithDepth,
        };

        /// Mock hierarchy that returns groups belonging to a specific tenant.
        struct MockHierarchy {
            tenant_id: Uuid,
        }

        #[async_trait]
        impl ResourceGroupReadHierarchy for MockHierarchy {
            async fn list_group_depth(
                &self,
                _ctx: &SecurityContext,
                group_id: Uuid,
                _query: ListQuery,
            ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
                Ok(Page {
                    items: vec![ResourceGroupWithDepth {
                        group_id,
                        parent_id: None,
                        group_type: "org".to_owned(),
                        name: "Test Group".to_owned(),
                        tenant_id: self.tenant_id,
                        external_id: None,
                        depth: 0,
                    }],
                    page_info: PageInfo { top: 10, skip: 0 },
                })
            }
        }

        /// Mock hierarchy that always returns an error.
        struct FailingHierarchy;

        #[async_trait]
        impl ResourceGroupReadHierarchy for FailingHierarchy {
            async fn list_group_depth(
                &self,
                _ctx: &SecurityContext,
                _group_id: Uuid,
                _query: ListQuery,
            ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
                Err(ResourceGroupError::Internal)
            }
        }

        /// Mock hierarchy that returns a group belonging to a DIFFERENT tenant.
        struct WrongTenantHierarchy;

        #[async_trait]
        impl ResourceGroupReadHierarchy for WrongTenantHierarchy {
            async fn list_group_depth(
                &self,
                _ctx: &SecurityContext,
                group_id: Uuid,
                _query: ListQuery,
            ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError> {
                Ok(Page {
                    items: vec![ResourceGroupWithDepth {
                        group_id,
                        parent_id: None,
                        group_type: "org".to_owned(),
                        name: "Other Tenant Group".to_owned(),
                        tenant_id: Uuid::parse_str("99999999-9999-9999-9999-999999999999")
                            .unwrap(),
                        external_id: None,
                        depth: 0,
                    }],
                    page_info: PageInfo { top: 10, skip: 0 },
                })
            }
        }

        fn make_group_hierarchy_request(
            tenant_id: Uuid,
            group_id: Uuid,
        ) -> EvaluationRequest {
            let mut subject_properties = HashMap::new();
            subject_properties.insert(
                "tenant_id".to_owned(),
                serde_json::Value::String(tenant_id.to_string()),
            );

            let mut resource_properties = HashMap::new();
            resource_properties.insert(
                "group_id".to_owned(),
                serde_json::Value::String(group_id.to_string()),
            );

            EvaluationRequest {
                subject: Subject {
                    id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
                    subject_type: None,
                    properties: subject_properties,
                },
                action: Action {
                    name: "list".to_owned(),
                },
                resource: Resource {
                    resource_type: "gts.cf.core.resource_group.group.v1".to_owned(),
                    id: None,
                    properties: resource_properties,
                },
                context: EvaluationRequestContext {
                    tenant_context: Some(TenantContext {
                        root_id: Some(tenant_id),
                        ..TenantContext::default()
                    }),
                    token_scopes: vec!["*".to_owned()],
                    require_constraints: true,
                    capabilities: vec![Capability::GroupHierarchy],
                    supported_properties: vec![],
                    bearer_token: None,
                },
            }
        }

        #[tokio::test]
        async fn group_hierarchy_allows_when_group_belongs_to_tenant() {
            let tenant_id =
                Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
            let group_id =
                Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();

            let hierarchy = Arc::new(MockHierarchy { tenant_id });
            let service = Service::with_hierarchy(hierarchy);

            let request = make_group_hierarchy_request(tenant_id, group_id);
            let response = service.evaluate(&request).await;

            assert!(response.decision);
            assert_eq!(response.context.constraints.len(), 1);
        }

        #[tokio::test]
        async fn group_hierarchy_denies_when_group_belongs_to_other_tenant() {
            let tenant_id =
                Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
            let group_id =
                Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();

            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> =
                Arc::new(WrongTenantHierarchy);
            let service = Service::with_hierarchy(hierarchy);

            let request = make_group_hierarchy_request(tenant_id, group_id);
            let response = service.evaluate(&request).await;

            // Should deny — group belongs to a different tenant
            assert!(!response.decision);
        }

        #[tokio::test]
        async fn group_hierarchy_fallback_on_error() {
            let tenant_id =
                Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
            let group_id =
                Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();

            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> =
                Arc::new(FailingHierarchy);
            let service = Service::with_hierarchy(hierarchy);

            let request = make_group_hierarchy_request(tenant_id, group_id);
            let response = service.evaluate(&request).await;

            // Graceful degradation: should still allow with tenant scope
            assert!(response.decision);
            assert_eq!(response.context.constraints.len(), 1);
        }

        #[tokio::test]
        async fn without_group_hierarchy_capability_skips_hierarchy_check() {
            let tenant_id =
                Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();

            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> =
                Arc::new(WrongTenantHierarchy);
            let service = Service::with_hierarchy(hierarchy);

            // Request WITHOUT GroupHierarchy capability
            let request = make_request(true, Some(tenant_id));
            let response = service.evaluate(&request).await;

            // Should allow even though hierarchy would deny — capability not declared
            assert!(response.decision);
        }

        #[tokio::test]
        async fn group_hierarchy_with_no_hierarchy_client_falls_back() {
            let tenant_id =
                Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
            let group_id =
                Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();

            // Service WITHOUT hierarchy client
            let service = Service::new();

            let request = make_group_hierarchy_request(tenant_id, group_id);
            let response = service.evaluate(&request).await;

            // Falls back to tenant-scoped constraint
            assert!(response.decision);
            assert_eq!(response.context.constraints.len(), 1);
        }
    }
}
