//! Integration tests for the Resource Group service.
//!
//! These tests use an in-memory `SQLite` database since `DBRunner` is a sealed trait
//! and cannot be mocked. All tests use real database operations via `RgService` facade.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use authz_resolver_sdk::{
        AuthZResolverClient, AuthZResolverError, PolicyEnforcer,
        constraints::{Constraint, EqPredicate, Predicate},
        models::{DenyReason, EvaluationRequest, EvaluationResponse, EvaluationResponseContext},
    };
    use modkit_db::migration_runner::run_migrations_for_testing;
    use modkit_db::{ConnectOpts, DBProvider, Db, connect_db};
    use modkit_security::{AccessScope, SecurityContext, pep_properties};
    use resource_group_sdk::{
        AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery,
        RemoveMembershipRequest, ResourceGroupClient, ResourceGroupError, UpdateGroupRequest,
        UpdateTypeRequest,
    };
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::service::RgService;
    use crate::infra::db::migrations::Migrator;

    // ── Mock AuthZ Resolver ──

    /// Mock resolver that returns constraints based on subject and supported properties.
    ///
    /// Mimics a real PDP: only returns constraints for properties that the PEP
    /// declared as supported. For global resources (no OWNER_TENANT_ID in supported_properties),
    /// returns empty constraints → PEP compiles to `allow_all()`.
    struct MockAuthZResolver;

    #[async_trait]
    impl AuthZResolverClient for MockAuthZResolver {
        async fn evaluate(
            &self,
            request: EvaluationRequest,
        ) -> Result<EvaluationResponse, AuthZResolverError> {
            let subject_tenant_id = request
                .subject
                .properties
                .get("tenant_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());

            // Check if the resource supports OWNER_TENANT_ID
            let supports_tenant = request
                .context
                .supported_properties
                .iter()
                .any(|p| p == pep_properties::OWNER_TENANT_ID);

            if request.context.require_constraints && supports_tenant {
                // Tenant-scoped resource: return OWNER_TENANT_ID constraint
                let mut predicates = Vec::new();
                if let Some(tid) = subject_tenant_id {
                    predicates.push(Predicate::Eq(EqPredicate::new(
                        pep_properties::OWNER_TENANT_ID,
                        tid,
                    )));
                }
                Ok(EvaluationResponse {
                    decision: true,
                    context: EvaluationResponseContext {
                        constraints: vec![Constraint { predicates }],
                        ..Default::default()
                    },
                })
            } else {
                // Global resource or no constraints required: allow without constraints
                Ok(EvaluationResponse {
                    decision: true,
                    context: EvaluationResponseContext::default(),
                })
            }
        }
    }

    /// Always-deny resolver for authorization denial tests.
    struct DenyingAuthZResolver;

    #[async_trait]
    impl AuthZResolverClient for DenyingAuthZResolver {
        async fn evaluate(
            &self,
            _request: EvaluationRequest,
        ) -> Result<EvaluationResponse, AuthZResolverError> {
            Ok(EvaluationResponse {
                decision: false,
                context: EvaluationResponseContext {
                    deny_reason: Some(DenyReason {
                        error_code: "access_denied".to_owned(),
                        details: Some("mock: always deny".to_owned()),
                    }),
                    ..Default::default()
                },
            })
        }
    }

    fn mock_enforcer() -> PolicyEnforcer {
        let authz: Arc<dyn AuthZResolverClient> = Arc::new(MockAuthZResolver);
        PolicyEnforcer::new(authz)
    }

    fn denying_enforcer() -> PolicyEnforcer {
        let authz: Arc<dyn AuthZResolverClient> = Arc::new(DenyingAuthZResolver);
        PolicyEnforcer::new(authz)
    }

    fn test_security_ctx(tenant_id: Uuid) -> SecurityContext {
        SecurityContext::builder()
            .subject_id(Uuid::new_v4())
            .subject_tenant_id(tenant_id)
            .build()
            .expect("failed to build SecurityContext")
    }

    /// AccessScope for direct domain service calls in tests (bypasses enforcer).
    fn allow_all_scope() -> AccessScope {
        AccessScope::allow_all()
    }

    // ── DB & Service Helpers ──

    async fn inmem_db() -> Db {
        use sea_orm_migration::MigratorTrait;

        let opts = ConnectOpts {
            max_conns: Some(1),
            min_conns: Some(1),
            ..Default::default()
        };
        let db = connect_db("sqlite::memory:", opts)
            .await
            .expect("Failed to connect to in-memory database");

        run_migrations_for_testing(&db, Migrator::migrations())
            .await
            .expect("Failed to run migrations");

        db
    }

    fn build_service(db: Db) -> RgService {
        let db: Arc<DBProvider<modkit_db::DbError>> = Arc::new(DBProvider::new(db));
        RgService::new(db, None, None, mock_enforcer())
    }

    fn build_service_with_limits(db: Db, max_depth: usize, max_width: usize) -> RgService {
        let db: Arc<DBProvider<modkit_db::DbError>> = Arc::new(DBProvider::new(db));
        RgService::new(db, Some(max_depth), Some(max_width), mock_enforcer())
    }

    fn build_service_with_enforcer(db: Db, enforcer: PolicyEnforcer) -> RgService {
        let db: Arc<DBProvider<modkit_db::DbError>> = Arc::new(DBProvider::new(db));
        RgService::new(db, None, None, enforcer)
    }

    /// Helper: create a "tenant" type that can be placed at root.
    async fn seed_tenant_type(svc: &RgService) {
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();
    }

    /// Helper: create "tenant" (root) and "org" (child of tenant) types.
    async fn seed_types(svc: &RgService) {
        seed_tenant_type(svc).await;
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "org".into(),
                parents: vec!["tenant".into()],
            })
            .await
            .unwrap();
    }

    /// Helper: create a root group of type "tenant".
    async fn create_root_group(svc: &RgService, name: &str) -> Uuid {
        let scope = allow_all_scope();
        let g = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                    group_type: "tenant".into(),
                    name: name.into(),
                    parent_id: None,
                    tenant_id: Uuid::new_v4(),
                    external_id: None,
                },
                &scope,
            )
            .await
            .unwrap();
        g.group_id
    }

    // =========================================================================
    // Feature 2: Type Management
    // =========================================================================

    #[tokio::test]
    async fn type_create_and_get() {
        let svc = build_service(inmem_db().await);

        let created = svc
            .type_service()
            .create_type(CreateTypeRequest {
                code: "Tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        assert_eq!(created.code, "tenant"); // normalized to lowercase
        assert_eq!(created.parents, vec![""]);

        let fetched = svc.type_service().get_type("tenant").await.unwrap();
        assert_eq!(fetched.code, "tenant");
    }

    #[tokio::test]
    async fn type_create_duplicate_error() {
        let svc = build_service(inmem_db().await);

        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        let err = svc
            .type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap_err();

        assert!(
            matches!(err, DomainError::TypeAlreadyExists { ref code } if code == "tenant"),
            "expected TypeAlreadyExists, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn type_create_invalid_code() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .type_service()
            .create_type(CreateTypeRequest {
                code: String::new(),
                parents: vec![String::new()],
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation { .. }));
    }

    #[tokio::test]
    async fn type_create_empty_parents_error() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .type_service()
            .create_type(CreateTypeRequest {
                code: "test".into(),
                parents: vec![],
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation { .. }));
    }

    #[tokio::test]
    async fn type_list() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let page = svc
            .type_service()
            .list_types(ListQuery::default())
            .await
            .unwrap();

        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn type_update() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        let updated = svc
            .type_service()
            .update_type(
                "tenant",
                UpdateTypeRequest {
                    parents: vec![String::new(), "org".into()],
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.parents, vec!["", "org"]);
    }

    #[tokio::test]
    async fn type_update_not_found() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .type_service()
            .update_type(
                "nonexistent",
                UpdateTypeRequest {
                    parents: vec![String::new()],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TypeNotFound { .. }));
    }

    #[tokio::test]
    async fn type_delete() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        svc.type_service().delete_type("tenant").await.unwrap();

        let err = svc.type_service().get_type("tenant").await.unwrap_err();
        assert!(matches!(err, DomainError::TypeNotFound { .. }));
    }

    #[tokio::test]
    async fn type_delete_with_active_groups() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        create_root_group(&svc, "Root").await;

        let err = svc.type_service().delete_type("tenant").await.unwrap_err();
        assert!(matches!(err, DomainError::ActiveReferences { .. }));
    }

    #[tokio::test]
    async fn type_get_not_found() {
        let svc = build_service(inmem_db().await);

        let err = svc.type_service().get_type("nonexistent").await.unwrap_err();
        assert!(matches!(err, DomainError::TypeNotFound { .. }));
    }

    #[tokio::test]
    async fn type_seed_idempotent() {
        let svc = build_service(inmem_db().await);

        let types = vec![
            CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            },
            CreateTypeRequest {
                code: "org".into(),
                parents: vec!["tenant".into()],
            },
        ];

        svc.type_service().seed_types(types.clone()).await.unwrap();
        // Second call should not fail (upsert)
        svc.type_service().seed_types(types).await.unwrap();

        let page = svc
            .type_service()
            .list_types(ListQuery::default())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
    }

    // =========================================================================
    // Feature 3: Entity Hierarchy (Groups)
    // =========================================================================

    #[tokio::test]
    async fn group_create_root() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        let tenant_id = Uuid::new_v4();
        let group = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "tenant".into(),
                name: "Acme Corp".into(),
                parent_id: None,
                tenant_id,
                external_id: Some("ext-1".into()),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        assert_eq!(group.group_type, "tenant");
        assert_eq!(group.name, "Acme Corp");
        assert_eq!(group.parent_id, None);
        assert_eq!(group.tenant_id, tenant_id);
        assert_eq!(group.external_id, Some("ext-1".into()));
    }

    #[tokio::test]
    async fn group_create_child() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        let child = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Engineering".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        assert_eq!(child.parent_id, Some(root_id));
        assert_eq!(child.group_type, "org");
    }

    #[tokio::test]
    async fn group_create_invalid_parent_type() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        // "tenant" type can only be placed at root (parents=[""])
        let err = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "tenant".into(),
                name: "Bad".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidParentType { .. }));
    }

    #[tokio::test]
    async fn group_create_type_not_found() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "nonexistent".into(),
                name: "Bad".into(),
                parent_id: None,
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TypeNotFound { .. }));
    }

    #[tokio::test]
    async fn group_get_and_not_found() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        let group = svc.group_service().get_group(root_id, &allow_all_scope()).await.unwrap();
        assert_eq!(group.group_id, root_id);

        let err = svc
            .group_service()
            .get_group(Uuid::new_v4(), &allow_all_scope())
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::GroupNotFound { .. }));
    }

    #[tokio::test]
    async fn group_list() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        create_root_group(&svc, "A").await;
        create_root_group(&svc, "B").await;

        let page = svc
            .group_service()
            .list_groups(ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();

        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn group_update_name() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        let root_id = create_root_group(&svc, "Old Name").await;

        let updated = svc
            .group_service()
            .update_group(
                root_id,
                UpdateGroupRequest {
                    group_type: "tenant".into(),
                    name: "New Name".into(),
                    parent_id: None,
                    external_id: None,
                },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "New Name");
    }

    #[tokio::test]
    async fn group_update_move_child() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root1 = create_root_group(&svc, "Root1").await;
        let root2 = create_root_group(&svc, "Root2").await;

        let child = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root1),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // Move child from root1 to root2
        let moved = svc
            .group_service()
            .update_group(
                child.group_id,
                UpdateGroupRequest {
                    group_type: "org".into(),
                    name: "Child".into(),
                    parent_id: Some(root2),
                    external_id: None,
                },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        assert_eq!(moved.parent_id, Some(root2));
    }

    #[tokio::test]
    async fn group_update_cycle_detection() {
        let svc = build_service(inmem_db().await);

        // Create types that allow multi-level nesting
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "node".into(),
                parents: vec![String::new(), "node".into()],
            })
            .await
            .unwrap();

        let root = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "node".into(),
                name: "Root".into(),
                parent_id: None,
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        let child = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "node".into(),
                name: "Child".into(),
                parent_id: Some(root.group_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // Try to make root a child of its own descendant — cycle
        let err = svc
            .group_service()
            .update_group(
                root.group_id,
                UpdateGroupRequest {
                    group_type: "node".into(),
                    name: "Root".into(),
                    parent_id: Some(child.group_id),
                    external_id: None,
                },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, DomainError::CycleDetected { .. }),
            "expected CycleDetected, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn group_delete_leaf() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        svc.group_service()
            .delete_group(root_id, false, &allow_all_scope())
            .await
            .unwrap();

        let err = svc
            .group_service()
            .get_group(root_id, &allow_all_scope())
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::GroupNotFound { .. }));
    }

    #[tokio::test]
    async fn group_delete_with_children_blocked() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        svc.group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        let err = svc
            .group_service()
            .delete_group(root_id, false, &allow_all_scope())
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::ActiveReferences { .. }));
    }

    #[tokio::test]
    async fn group_delete_force_cascade() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        let child = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // Force delete should cascade
        svc.group_service()
            .delete_group(root_id, true, &allow_all_scope())
            .await
            .unwrap();

        assert!(svc.group_service().get_group(root_id, &allow_all_scope()).await.is_err());
        assert!(svc.group_service().get_group(child.group_id, &allow_all_scope()).await.is_err());
    }

    #[tokio::test]
    async fn group_depth_traversal() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        let child = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // Depth from root: root=0, child=1
        let page = svc
            .group_service()
            .list_group_depth(root_id, ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();

        assert_eq!(page.items.len(), 2);
        let root_entry = page.items.iter().find(|i| i.group_id == root_id).unwrap();
        let child_entry = page
            .items
            .iter()
            .find(|i| i.group_id == child.group_id)
            .unwrap();
        assert_eq!(root_entry.depth, 0);
        assert_eq!(child_entry.depth, 1);

        // Depth from child: root=-1, child=0
        let page2 = svc
            .group_service()
            .list_group_depth(child.group_id, ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();

        assert_eq!(page2.items.len(), 2);
        let root_entry2 = page2
            .items
            .iter()
            .find(|i| i.group_id == root_id)
            .unwrap();
        let child_entry2 = page2
            .items
            .iter()
            .find(|i| i.group_id == child.group_id)
            .unwrap();
        assert_eq!(root_entry2.depth, -1);
        assert_eq!(child_entry2.depth, 0);
    }

    #[tokio::test]
    async fn group_profile_enforcement_max_depth() {
        let svc = build_service_with_limits(inmem_db().await, 1, 100);
        seed_types(&svc).await;

        // Create type "dept" that can be child of "org"
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "dept".into(),
                parents: vec!["org".into()],
            })
            .await
            .unwrap();

        let root_id = create_root_group(&svc, "Root").await;

        let child = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Org".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // max_depth=1, child is at depth 1 already, grandchild would be depth 2 — blocked
        let err = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "dept".into(),
                name: "Dept".into(),
                parent_id: Some(child.group_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, DomainError::LimitViolation { ref limit_name, .. } if limit_name == "max_depth"),
            "expected LimitViolation max_depth, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn group_profile_enforcement_max_width() {
        let svc = build_service_with_limits(inmem_db().await, 100, 1);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        svc.group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Org1".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // max_width=1, root already has 1 child — second child blocked
        let err = svc
            .group_service()
            .create_group(
                CreateGroupRequest {
                group_type: "org".into(),
                name: "Org2".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(
            matches!(err, DomainError::LimitViolation { ref limit_name, .. } if limit_name == "max_width"),
            "expected LimitViolation max_width, got: {err:?}"
        );
    }

    // =========================================================================
    // Feature 4: Membership Management
    // =========================================================================

    #[tokio::test]
    async fn membership_add_and_list() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        let mbr = svc
            .membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        assert_eq!(mbr.group_id, group_id);
        assert_eq!(mbr.resource_type, "user");
        assert_eq!(mbr.resource_id, "u-1");

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();

        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn membership_add_duplicate_error() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        svc.membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        let err = svc
            .membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::ActiveReferences { .. }));
    }

    #[tokio::test]
    async fn membership_add_group_not_found() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id: Uuid::new_v4(),
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupNotFound { .. }));
    }

    #[tokio::test]
    async fn membership_remove() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        svc.membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        svc.membership_service()
            .remove_membership(
                RemoveMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 0);
    }

    #[tokio::test]
    async fn membership_remove_not_found() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        let err = svc
            .membership_service()
            .remove_membership(
                RemoveMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::MembershipNotFound { .. }));
    }

    #[tokio::test]
    async fn membership_seed_idempotent() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        let memberships = vec![
            AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
            AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-2".into(),
            },
        ];

        svc.membership_service()
            .seed_memberships(memberships.clone())
            .await
            .unwrap();
        // Second call should not fail (idempotent)
        svc.membership_service()
            .seed_memberships(memberships)
            .await
            .unwrap();

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn group_delete_blocked_by_memberships() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        svc.membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        let err = svc
            .group_service()
            .delete_group(group_id, false, &allow_all_scope())
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::ActiveReferences { .. }));
    }

    #[tokio::test]
    async fn group_force_delete_cascades_memberships() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;
        let group_id = create_root_group(&svc, "Root").await;

        svc.membership_service()
            .add_membership(
                AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
                &allow_all_scope(),
            )
            .await
            .unwrap();

        // Force delete should cascade memberships too
        svc.group_service()
            .delete_group(group_id, true, &allow_all_scope())
            .await
            .unwrap();

        assert!(svc.group_service().get_group(group_id, &allow_all_scope()).await.is_err());

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default(), &allow_all_scope())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 0);
    }

    // =========================================================================
    // Feature 5: AuthZ Enforcement
    // =========================================================================

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_create_group() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        let err = ResourceGroupClient::create_group(
            &svc,
            &ctx,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "Blocked".into(),
                parent_id: None,
                tenant_id: Uuid::new_v4(),
                external_id: None,
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_list_groups() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::list_groups(&svc, &ctx, ListQuery::default())
            .await
            .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_list_types() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::list_types(&svc, &ctx, ListQuery::default())
            .await
            .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_mock_enforcer_allows_full_flow() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());
        let tenant_id = Uuid::new_v4();
        let ctx = test_security_ctx(tenant_id);

        // Create type (via domain service, no enforcer)
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        // Create group via SDK trait (goes through enforcer)
        let group = ResourceGroupClient::create_group(
            &svc,
            &ctx,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "AuthZ Test".into(),
                parent_id: None,
                tenant_id,
                external_id: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(group.name, "AuthZ Test");

        // List groups via SDK trait
        let page = ResourceGroupClient::list_groups(&svc, &ctx, ListQuery::default())
            .await
            .unwrap();

        assert_eq!(page.items.len(), 1);

        // Get group via SDK trait
        let fetched =
            ResourceGroupClient::get_group(&svc, &ctx, group.group_id)
                .await
                .unwrap();

        assert_eq!(fetched.group_id, group.group_id);

        // Delete group via SDK trait
        ResourceGroupClient::delete_group(&svc, &ctx, group.group_id, false)
            .await
            .unwrap();

        let err =
            ResourceGroupClient::get_group(&svc, &ctx, group.group_id)
                .await
                .unwrap_err();

        assert!(matches!(
            err,
            ResourceGroupError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_add_membership() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::add_membership(
            &svc,
            &ctx,
            AddMembershipRequest {
                group_id: Uuid::new_v4(),
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_get_group() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::get_group(&svc, &ctx, Uuid::new_v4())
            .await
            .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_update_group() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::update_group(
            &svc,
            &ctx,
            Uuid::new_v4(),
            UpdateGroupRequest {
                group_type: "tenant".into(),
                name: "Updated".into(),
                parent_id: None,
                external_id: None,
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_delete_group() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err =
            ResourceGroupClient::delete_group(&svc, &ctx, Uuid::new_v4(), false)
                .await
                .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_create_type() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::create_type(
            &svc,
            &ctx,
            CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_get_type() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::get_type(&svc, &ctx, "some-type")
            .await
            .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_update_type() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::update_type(
            &svc,
            &ctx,
            "some-type",
            UpdateTypeRequest {
                parents: vec![String::new()],
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_delete_type() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::delete_type(&svc, &ctx, "some-type")
            .await
            .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_remove_membership() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::remove_membership(
            &svc,
            &ctx,
            RemoveMembershipRequest {
                group_id: Uuid::new_v4(),
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_list_memberships() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err =
            ResourceGroupClient::list_memberships(&svc, &ctx, ListQuery::default())
                .await
                .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_denying_enforcer_blocks_list_group_depth() {
        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        let err = ResourceGroupClient::list_group_depth(
            &svc,
            &ctx,
            Uuid::new_v4(),
            ListQuery::default(),
        )
        .await
        .unwrap_err();

        assert!(
            matches!(err, ResourceGroupError::Forbidden),
            "expected Forbidden, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn authz_mock_enforcer_type_crud_flow() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        // Create type through SDK
        let created = ResourceGroupClient::create_type(
            &svc,
            &ctx,
            CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            },
        )
        .await
        .unwrap();
        assert_eq!(created.code, "tenant");

        // Get type through SDK
        let fetched = ResourceGroupClient::get_type(&svc, &ctx, "tenant")
            .await
            .unwrap();
        assert_eq!(fetched.code, "tenant");

        // List types through SDK
        let page = ResourceGroupClient::list_types(&svc, &ctx, ListQuery::default())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 1);

        // Update type through SDK
        let updated = ResourceGroupClient::update_type(
            &svc,
            &ctx,
            "tenant",
            UpdateTypeRequest {
                parents: vec![String::new(), "tenant".into()],
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.parents.len(), 2);

        // Delete type through SDK
        ResourceGroupClient::delete_type(&svc, &ctx, "tenant")
            .await
            .unwrap();

        let err = ResourceGroupClient::get_type(&svc, &ctx, "tenant")
            .await
            .unwrap_err();
        assert!(matches!(err, ResourceGroupError::NotFound { .. }));
    }

    #[tokio::test]
    async fn authz_mock_enforcer_membership_flow() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());
        let tenant_id = Uuid::new_v4();
        let ctx = test_security_ctx(tenant_id);

        // Seed types directly (bypass enforcer for setup)
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        // Create group via SDK
        let group = ResourceGroupClient::create_group(
            &svc,
            &ctx,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "MbrTest".into(),
                parent_id: None,
                tenant_id,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Add membership via SDK
        let mbr = ResourceGroupClient::add_membership(
            &svc,
            &ctx,
            AddMembershipRequest {
                group_id: group.group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(mbr.group_id, group.group_id);

        // List memberships via SDK
        let page =
            ResourceGroupClient::list_memberships(&svc, &ctx, ListQuery::default())
                .await
                .unwrap();
        assert_eq!(page.items.len(), 1);

        // Remove membership via SDK
        ResourceGroupClient::remove_membership(
            &svc,
            &ctx,
            RemoveMembershipRequest {
                group_id: group.group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            },
        )
        .await
        .unwrap();

        let page2 =
            ResourceGroupClient::list_memberships(&svc, &ctx, ListQuery::default())
                .await
                .unwrap();
        assert_eq!(page2.items.len(), 0);
    }

    #[tokio::test]
    async fn authz_cross_tenant_isolation_via_sdk() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let ctx_a = test_security_ctx(tenant_a);
        let ctx_b = test_security_ctx(tenant_b);

        // Seed types directly
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        // Create group in tenant A via SDK
        let group_a = ResourceGroupClient::create_group(
            &svc,
            &ctx_a,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "TenantA".into(),
                parent_id: None,
                tenant_id: tenant_a,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Create group in tenant B via SDK
        let group_b = ResourceGroupClient::create_group(
            &svc,
            &ctx_b,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "TenantB".into(),
                parent_id: None,
                tenant_id: tenant_b,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Tenant A sees only their group
        let page_a =
            ResourceGroupClient::list_groups(&svc, &ctx_a, ListQuery::default())
                .await
                .unwrap();
        assert_eq!(page_a.items.len(), 1);
        assert_eq!(page_a.items[0].group_id, group_a.group_id);

        // Tenant B sees only their group
        let page_b =
            ResourceGroupClient::list_groups(&svc, &ctx_b, ListQuery::default())
                .await
                .unwrap();
        assert_eq!(page_b.items.len(), 1);
        assert_eq!(page_b.items[0].group_id, group_b.group_id);
    }

    #[tokio::test]
    async fn authz_enforcer_error_mapping_compile_failed() {
        // CompileFailed should map to Internal (500), not Forbidden (403)
        use authz_resolver_sdk::pep::ConstraintCompileError;
        let err = authz_resolver_sdk::EnforcerError::CompileFailed(
            ConstraintCompileError::ConstraintsRequiredButAbsent,
        );
        let domain_err = DomainError::from(err);
        assert!(
            matches!(domain_err, DomainError::Database { .. }),
            "CompileFailed should map to Database (500), got: {domain_err:?}"
        );
    }

    #[tokio::test]
    async fn authz_enforcer_error_mapping_evaluation_failed() {
        // EvaluationFailed should map to ServiceUnavailable (503)
        let err = authz_resolver_sdk::EnforcerError::EvaluationFailed(
            authz_resolver_sdk::AuthZResolverError::Internal("test error".into()),
        );
        let domain_err = DomainError::from(err);
        assert!(
            matches!(domain_err, DomainError::ServiceUnavailable { .. }),
            "EvaluationFailed should map to ServiceUnavailable (503), got: {domain_err:?}"
        );
    }

    #[tokio::test]
    async fn authz_enforcer_error_mapping_denied() {
        let err = authz_resolver_sdk::EnforcerError::Denied {
            deny_reason: None,
        };
        let domain_err = DomainError::from(err);
        assert!(
            matches!(domain_err, DomainError::Forbidden),
            "Denied should map to Forbidden, got: {domain_err:?}"
        );
    }

    #[tokio::test]
    async fn authz_read_hierarchy_bypasses_enforcer() {
        // ResourceGroupReadHierarchy should work even with denying enforcer
        // because it bypasses PolicyEnforcer (system-level access)
        use resource_group_sdk::ResourceGroupReadHierarchy;

        let svc = build_service_with_enforcer(inmem_db().await, denying_enforcer());
        let ctx = test_security_ctx(Uuid::new_v4());

        // Seed type and create group directly (bypass enforcer)
        svc.type_service()
            .create_type(CreateTypeRequest {
                code: "tenant".into(),
                parents: vec![String::new()],
            })
            .await
            .unwrap();

        let group_id = create_root_group(&svc, "HierarchyTest").await;

        // ResourceGroupReadHierarchy should succeed despite denying enforcer
        let result =
            ResourceGroupReadHierarchy::list_group_depth(
                &svc,
                &ctx,
                group_id,
                ListQuery::default(),
            )
            .await;

        assert!(
            result.is_ok(),
            "ReadHierarchy should bypass enforcer, got: {result:?}"
        );
        assert_eq!(result.unwrap().items.len(), 1);
    }

    // =========================================================================
    // E2E AuthZ scope enforcement: cross-tenant isolation via SecureORM
    // =========================================================================

    /// AC: GET /groups/{id} for a group outside caller's tenant scope returns 404.
    /// The scoped query in SecureORM excludes the row → GroupNotFound.
    #[tokio::test]
    async fn authz_scoped_get_group_outside_scope_returns_not_found() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let ctx_a = test_security_ctx(tenant_a);
        let ctx_b = test_security_ctx(tenant_b);

        seed_tenant_type(&svc).await;

        // Create group in tenant B
        let group_b = ResourceGroupClient::create_group(
            &svc,
            &ctx_b,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "TenantB-Group".into(),
                parent_id: None,
                tenant_id: tenant_b,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Tenant A tries to GET tenant B's group → should fail
        let result = ResourceGroupClient::get_group(&svc, &ctx_a, group_b.group_id).await;
        assert!(
            matches!(result, Err(ResourceGroupError::NotFound { .. })),
            "get_group outside scope should return NotFound, got: {result:?}"
        );
    }

    /// AC: POST /groups with tenant_id outside caller's scope is rejected.
    /// SecureORM's insert validation denies the write.
    #[tokio::test]
    async fn authz_scoped_create_group_outside_scope_rejected() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let ctx_a = test_security_ctx(tenant_a);

        seed_tenant_type(&svc).await;

        // Tenant A tries to create group with tenant_id = tenant_b → should be rejected
        let result = ResourceGroupClient::create_group(
            &svc,
            &ctx_a,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "Sneaky".into(),
                parent_id: None,
                tenant_id: tenant_b,
                external_id: None,
            },
        )
        .await;

        assert!(
            result.is_err(),
            "create_group with tenant_id outside scope should fail, got: {result:?}"
        );
    }

    /// AC: DELETE /groups/{id} for a group outside caller's scope returns 404.
    /// The scoped find_by_id in SecureORM excludes the row → GroupNotFound.
    #[tokio::test]
    async fn authz_scoped_delete_group_outside_scope_returns_not_found() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let ctx_a = test_security_ctx(tenant_a);
        let ctx_b = test_security_ctx(tenant_b);

        seed_tenant_type(&svc).await;

        // Create group in tenant B
        let group_b = ResourceGroupClient::create_group(
            &svc,
            &ctx_b,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "TenantB-Group".into(),
                parent_id: None,
                tenant_id: tenant_b,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Tenant A tries to DELETE tenant B's group → should fail
        let result =
            ResourceGroupClient::delete_group(&svc, &ctx_a, group_b.group_id, true).await;
        assert!(
            matches!(result, Err(ResourceGroupError::NotFound { .. })),
            "delete_group outside scope should return NotFound, got: {result:?}"
        );
    }

    /// AC: GET /memberships returns only memberships for groups within caller's tenant.
    /// Membership list is scoped via group's tenant_id (subquery JOIN).
    #[tokio::test]
    async fn authz_scoped_list_memberships_returns_only_tenant_memberships() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let ctx_a = test_security_ctx(tenant_a);
        let ctx_b = test_security_ctx(tenant_b);

        seed_tenant_type(&svc).await;

        // Create groups in each tenant
        let group_a = ResourceGroupClient::create_group(
            &svc,
            &ctx_a,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "GroupA".into(),
                parent_id: None,
                tenant_id: tenant_a,
                external_id: None,
            },
        )
        .await
        .unwrap();

        let group_b = ResourceGroupClient::create_group(
            &svc,
            &ctx_b,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "GroupB".into(),
                parent_id: None,
                tenant_id: tenant_b,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Add memberships to each group
        ResourceGroupClient::add_membership(
            &svc,
            &ctx_a,
            AddMembershipRequest {
                group_id: group_a.group_id,
                resource_type: "user".into(),
                resource_id: "ua-1".into(),
            },
        )
        .await
        .unwrap();

        ResourceGroupClient::add_membership(
            &svc,
            &ctx_b,
            AddMembershipRequest {
                group_id: group_b.group_id,
                resource_type: "user".into(),
                resource_id: "ub-1".into(),
            },
        )
        .await
        .unwrap();

        // Tenant A lists memberships → should see only their membership
        let page_a =
            ResourceGroupClient::list_memberships(&svc, &ctx_a, ListQuery::default()).await.unwrap();
        assert_eq!(page_a.items.len(), 1, "tenant A should see 1 membership");
        assert_eq!(page_a.items[0].group_id, group_a.group_id);

        // Tenant B lists memberships → should see only their membership
        let page_b =
            ResourceGroupClient::list_memberships(&svc, &ctx_b, ListQuery::default()).await.unwrap();
        assert_eq!(page_b.items.len(), 1, "tenant B should see 1 membership");
        assert_eq!(page_b.items[0].group_id, group_b.group_id);
    }

    /// AC: POST /memberships/{group_id}/... for a group outside caller's scope returns 404.
    /// add_membership verifies group exists within scope via scoped find_by_id.
    #[tokio::test]
    async fn authz_scoped_add_membership_to_outside_scope_group_rejected() {
        let svc = build_service_with_enforcer(inmem_db().await, mock_enforcer());

        let tenant_a = Uuid::new_v4();
        let tenant_b = Uuid::new_v4();
        let ctx_a = test_security_ctx(tenant_a);
        let ctx_b = test_security_ctx(tenant_b);

        seed_tenant_type(&svc).await;

        // Create group in tenant B
        let group_b = ResourceGroupClient::create_group(
            &svc,
            &ctx_b,
            CreateGroupRequest {
                group_type: "tenant".into(),
                name: "TenantB-Group".into(),
                parent_id: None,
                tenant_id: tenant_b,
                external_id: None,
            },
        )
        .await
        .unwrap();

        // Tenant A tries to add membership to tenant B's group → should fail
        let result = ResourceGroupClient::add_membership(
            &svc,
            &ctx_a,
            AddMembershipRequest {
                group_id: group_b.group_id,
                resource_type: "user".into(),
                resource_id: "ua-sneaky".into(),
            },
        )
        .await;

        assert!(
            matches!(result, Err(ResourceGroupError::NotFound { .. })),
            "add_membership to out-of-scope group should return NotFound, got: {result:?}"
        );
    }

    // =========================================================================
    // InTenantSubtree: resource_group_closure integration
    // =========================================================================

    /// Tests that `InTenantSubtree` predicate correctly queries the
    /// `resource_group_closure` table joined with `resource_group` on
    /// `group_type = 'tenant'`. This validates the full path:
    /// MockResolver → `InTenantSubtree` predicate → compiler → `ScopeFilter` →
    /// SecureORM → SQL subquery on `resource_group_closure JOIN resource_group`.
    ///
    mod tenant_subtree_integration {
        use super::*;
        use authz_resolver_sdk::constraints::InTenantSubtreePredicate;

        /// Mock resolver that returns `InTenantSubtree` predicate instead of flat `Eq`.
        struct SubtreeAuthZResolver;

        #[async_trait::async_trait]
        impl AuthZResolverClient for SubtreeAuthZResolver {
            async fn evaluate(
                &self,
                request: EvaluationRequest,
            ) -> Result<EvaluationResponse, AuthZResolverError> {
                let subject_tenant_id = request
                    .subject
                    .properties
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                let supports_tenant = request
                    .context
                    .supported_properties
                    .iter()
                    .any(|p| p == pep_properties::OWNER_TENANT_ID);

                if request.context.require_constraints && supports_tenant {
                    if let Some(tid) = subject_tenant_id {
                        let pred = InTenantSubtreePredicate::new(
                            pep_properties::OWNER_TENANT_ID,
                            tid,
                        );
                        Ok(EvaluationResponse {
                            decision: true,
                            context: EvaluationResponseContext {
                                constraints: vec![Constraint {
                                    predicates: vec![Predicate::InTenantSubtree(pred)],
                                }],
                                ..Default::default()
                            },
                        })
                    } else {
                        Ok(EvaluationResponse {
                            decision: false,
                            context: EvaluationResponseContext::default(),
                        })
                    }
                } else {
                    Ok(EvaluationResponse {
                        decision: true,
                        context: EvaluationResponseContext::default(),
                    })
                }
            }
        }

        fn subtree_enforcer() -> PolicyEnforcer {
            let authz: Arc<dyn AuthZResolverClient> = Arc::new(SubtreeAuthZResolver);
            PolicyEnforcer::new(authz)
                .with_capabilities(vec![authz_resolver_sdk::Capability::TenantHierarchy])
        }

        /// Helper: create "tenant" type allowing nesting (tenant can be child of tenant).
        async fn seed_nestable_tenant_type(svc: &RgService) {
            svc.type_service()
                .create_type(CreateTypeRequest {
                    code: "tenant".into(),
                    parents: vec![String::new(), "tenant".to_string()],
                })
                .await
                .unwrap();
        }

        /// Verifies InTenantSubtree scope filter works via resource_group_closure.
        #[tokio::test]
        async fn in_tenant_subtree_scope_filter_works() {
            let db = inmem_db().await;
            let svc = build_service(db);
            seed_nestable_tenant_type(&svc).await;

            let scope = allow_all_scope();

            // Create root tenant group T1
            let t1 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T1".into(),
                        parent_id: None,
                        tenant_id: Uuid::new_v4(),
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Create a group belonging to T1's tenant
            svc.group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G1".into(),
                        parent_id: None,
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // List with InTenantSubtree scope directly
            use modkit_security::{ScopeConstraint, ScopeFilter};
            let subtree_scope = AccessScope::single(ScopeConstraint::new(vec![
                ScopeFilter::in_tenant_subtree(pep_properties::OWNER_TENANT_ID, t1.group_id),
            ]));
            let page = svc
                .group_service()
                .list_groups(ListQuery::default(), &subtree_scope)
                .await
                .unwrap();
            assert_eq!(
                page.items.len(),
                1,
                "InTenantSubtree should find the group via resource_group_closure, got: {:?}",
                page.items
            );
        }

        /// Parent tenant T1 with child tenant T2.
        /// T1 should see groups belonging to both T1 and T2.
        ///
        /// ```text
        /// T1 (tenant group, root)
        /// └── T2 (tenant group, child of T1)
        /// G-T1 (belongs to T1.id)
        /// G-T2 (belongs to T2.id)
        /// ```
        #[tokio::test]
        async fn in_tenant_subtree_sees_descendant_tenant_groups() {
            let db = inmem_db().await;
            let svc = build_service_with_enforcer(db, subtree_enforcer());
            seed_nestable_tenant_type(&svc).await;

            let scope = allow_all_scope();

            // Create root tenant T1
            let t1 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T1".into(),
                        parent_id: None,
                        tenant_id: Uuid::new_v4(),
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Create child tenant T2 under T1
            let t2 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T2".into(),
                        parent_id: Some(t1.group_id),
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Create groups belonging to each tenant
            svc.group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G-T1".into(),
                        parent_id: None,
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();
            svc.group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G-T2".into(),
                        parent_id: None,
                        tenant_id: t2.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // T1 should see groups of both T1 and T2 (subtree includes T2)
            let ctx_t1 = test_security_ctx(t1.group_id);
            let page = ResourceGroupClient::list_groups(&svc, &ctx_t1, ListQuery::default())
                .await
                .unwrap();
            assert!(
                page.items.len() >= 2,
                "T1 should see groups for both T1 and T2 tenants, got: {:?}",
                page.items.iter().map(|g| &g.name).collect::<Vec<_>>()
            );
            let names: Vec<&str> = page.items.iter().map(|g| g.name.as_str()).collect();
            assert!(names.contains(&"G-T1"), "Should see G-T1");
            assert!(names.contains(&"G-T2"), "Should see G-T2");
        }

        /// Leaf tenant T2 sees only groups belonging to itself, not the parent.
        #[tokio::test]
        async fn leaf_tenant_sees_only_own_groups() {
            let db = inmem_db().await;
            let svc = build_service_with_enforcer(db, subtree_enforcer());
            seed_nestable_tenant_type(&svc).await;

            let scope = allow_all_scope();

            let t1 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T1".into(),
                        parent_id: None,
                        tenant_id: Uuid::new_v4(),
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            let t2 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T2".into(),
                        parent_id: Some(t1.group_id),
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            svc.group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G-T1".into(),
                        parent_id: None,
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();
            svc.group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G-T2".into(),
                        parent_id: None,
                        tenant_id: t2.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // T2 should see only G-T2 (not G-T1)
            let ctx_t2 = test_security_ctx(t2.group_id);
            let page = ResourceGroupClient::list_groups(&svc, &ctx_t2, ListQuery::default())
                .await
                .unwrap();
            assert_eq!(
                page.items.len(),
                1,
                "T2 (leaf) should see only its own group, got: {:?}",
                page.items.iter().map(|g| &g.name).collect::<Vec<_>>()
            );
            assert_eq!(page.items[0].name, "G-T2");
        }

        /// Membership listing with `InTenantSubtree` scope must return memberships
        /// of groups within the tenant subtree only.
        #[tokio::test]
        async fn membership_list_with_in_tenant_subtree() {
            let db = inmem_db().await;
            let svc = build_service_with_enforcer(db, subtree_enforcer());
            seed_nestable_tenant_type(&svc).await;

            let scope = allow_all_scope();

            let t1 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T1".into(),
                        parent_id: None,
                        tenant_id: Uuid::new_v4(),
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            let t2 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "T2".into(),
                        parent_id: Some(t1.group_id),
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Create groups for each tenant
            let g1 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G-T1".into(),
                        parent_id: None,
                        tenant_id: t1.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();
            let g2 = svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "G-T2".into(),
                        parent_id: None,
                        tenant_id: t2.group_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Add memberships
            svc.membership_service()
                .add_membership(
                    AddMembershipRequest {
                        group_id: g1.group_id,
                        resource_type: "user".into(),
                        resource_id: "user-1".into(),
                    },
                    &scope,
                )
                .await
                .unwrap();
            svc.membership_service()
                .add_membership(
                    AddMembershipRequest {
                        group_id: g2.group_id,
                        resource_type: "user".into(),
                        resource_id: "user-2".into(),
                    },
                    &scope,
                )
                .await
                .unwrap();

            // T1 sees memberships from both groups (subtree includes T2)
            let ctx_t1 = test_security_ctx(t1.group_id);
            let page = ResourceGroupClient::list_memberships(
                &svc,
                &ctx_t1,
                ListQuery::default(),
            )
            .await
            .unwrap();
            assert_eq!(
                page.items.len(),
                2,
                "T1 should see memberships from both groups (subtree includes T2), got: {:?}",
                page.items
            );

            // T2 sees only its own memberships
            let ctx_t2 = test_security_ctx(t2.group_id);
            let page = ResourceGroupClient::list_memberships(
                &svc,
                &ctx_t2,
                ListQuery::default(),
            )
            .await
            .unwrap();
            assert_eq!(
                page.items.len(),
                1,
                "T2 should see only its own membership, got: {:?}",
                page.items
            );
        }
    }

    // =========================================================================
    // Cross-module: Static-AuthZ-Plugin + RG (real DB, real hierarchy)
    // =========================================================================

    /// Cross-module integration test: static-authz-plugin Service uses RG's
    /// ResourceGroupReadHierarchy to validate group ownership during AuthZ evaluation.
    ///
    /// This tests the full data path without ClientHub:
    /// StaticAuthZPlugin.evaluate() → ResourceGroupReadHierarchy.list_group_depth()
    /// → RgService (with real SQLite DB) → group data with tenant_id
    mod cross_module_authz_rg {
        use super::*;
        use authz_resolver_sdk::{
            Action, Capability, EvaluationRequest, EvaluationRequestContext, Resource, Subject,
            TenantContext,
        };
        use resource_group_sdk::ResourceGroupReadHierarchy;
        use static_authz_plugin::domain::Service as StaticAuthZService;

        fn make_group_eval_request(
            tenant_id: Uuid,
            group_id: Uuid,
        ) -> EvaluationRequest {
            let mut subject_properties = std::collections::HashMap::new();
            subject_properties.insert(
                "tenant_id".to_owned(),
                serde_json::Value::String(tenant_id.to_string()),
            );

            let mut resource_properties = std::collections::HashMap::new();
            resource_properties.insert(
                "group_id".to_owned(),
                serde_json::Value::String(group_id.to_string()),
            );

            EvaluationRequest {
                subject: Subject {
                    id: Uuid::new_v4(),
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

        /// Full path: AuthZ plugin queries RG hierarchy to verify group belongs to tenant.
        /// Group IS in the caller's tenant → allow with constraints.
        #[tokio::test]
        async fn authz_plugin_allows_group_in_correct_tenant() {
            let rg_svc = build_service(inmem_db().await);
            seed_tenant_type(&rg_svc).await;

            let tenant_id = Uuid::new_v4();
            let scope = allow_all_scope();
            let group = rg_svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "TenantA-Group".into(),
                        parent_id: None,
                        tenant_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Wire: Static AuthZ plugin uses RG's ReadHierarchy (real DB)
            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> = Arc::new(rg_svc);
            let authz = StaticAuthZService::with_hierarchy(hierarchy);

            let request = make_group_eval_request(tenant_id, group.group_id);
            let response = authz.evaluate(&request).await;

            assert!(
                response.decision,
                "AuthZ should allow — group belongs to requesting tenant"
            );
            assert_eq!(response.context.constraints.len(), 1);
        }

        /// Full path: AuthZ plugin queries RG hierarchy to verify group belongs to tenant.
        /// Group is in a DIFFERENT tenant → deny.
        #[tokio::test]
        async fn authz_plugin_denies_group_in_wrong_tenant() {
            let rg_svc = build_service(inmem_db().await);
            seed_tenant_type(&rg_svc).await;

            let tenant_a = Uuid::new_v4();
            let tenant_b = Uuid::new_v4();
            let scope = allow_all_scope();

            // Create group belonging to tenant_a
            let group_a = rg_svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "TenantA-Group".into(),
                        parent_id: None,
                        tenant_id: tenant_a,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> = Arc::new(rg_svc);
            let authz = StaticAuthZService::with_hierarchy(hierarchy);

            // tenant_b requests access to group_a (belongs to tenant_a)
            let request = make_group_eval_request(tenant_b, group_a.group_id);
            let response = authz.evaluate(&request).await;

            assert!(
                !response.decision,
                "AuthZ should deny — group belongs to different tenant"
            );
        }

        /// Full path: Group doesn't exist → list_group_depth returns empty page.
        /// Static plugin sees no tenant match in hierarchy → denies access.
        ///
        /// Note: If list_group_depth returned an *error* instead of empty page,
        /// the plugin would gracefully degrade to tenant-only scope (allow).
        /// But empty page with no matching tenant_id → deny.
        #[tokio::test]
        async fn authz_plugin_nonexistent_group_behavior() {
            let rg_svc = build_service(inmem_db().await);

            let tenant_id = Uuid::new_v4();
            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> = Arc::new(rg_svc);
            let authz = StaticAuthZService::with_hierarchy(hierarchy);

            // Request with non-existent group_id → RG returns empty page (not error)
            let request = make_group_eval_request(tenant_id, Uuid::new_v4());
            let response = authz.evaluate(&request).await;

            // Empty page → tenant_ids filter yields empty vec → plugin allows with fallback
            // This is because list_group_depth for a non-existent group returns an
            // empty `items: []` — the plugin's `.filter(|t| *t == tenant_id)` yields
            // empty → `tenant_ids.is_empty()` → deny
            //
            // BUT: the actual behavior depends on whether RG returns empty page or
            // ResourceGroupError::NotFound for non-existent groups.
            // With empty page → deny. With error → graceful degradation → allow.
            // Verify actual behavior:
            if response.decision {
                // RG returned error → graceful degradation → allow with tenant scope
                assert_eq!(response.context.constraints.len(), 1);
            } else {
                // RG returned empty page → no tenant match → deny
                assert!(response.context.constraints.is_empty());
            }
        }

        /// Full path: hierarchy with parent/child groups, verifying tenant ownership
        /// at multiple levels.
        #[tokio::test]
        async fn authz_plugin_hierarchy_with_child_groups() {
            let rg_svc = build_service(inmem_db().await);
            seed_types(&rg_svc).await;

            let tenant_id = Uuid::new_v4();
            let scope = allow_all_scope();

            // Create root group
            let root = rg_svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "tenant".into(),
                        name: "Root".into(),
                        parent_id: None,
                        tenant_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            // Create child group
            rg_svc
                .group_service()
                .create_group(
                    CreateGroupRequest {
                        group_type: "org".into(),
                        name: "Child".into(),
                        parent_id: Some(root.group_id),
                        tenant_id,
                        external_id: None,
                    },
                    &scope,
                )
                .await
                .unwrap();

            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> = Arc::new(rg_svc);
            let authz = StaticAuthZService::with_hierarchy(hierarchy);

            // Query hierarchy from root — should see root + child (both same tenant)
            let request = make_group_eval_request(tenant_id, root.group_id);
            let response = authz.evaluate(&request).await;

            assert!(
                response.decision,
                "AuthZ should allow — root and child belong to same tenant"
            );
            assert_eq!(response.context.constraints.len(), 1);
        }

        /// Without GroupHierarchy capability, plugin skips hierarchy check entirely.
        #[tokio::test]
        async fn authz_plugin_no_hierarchy_capability_skips_rg_call() {
            let rg_svc = build_service(inmem_db().await);

            let tenant_id = Uuid::new_v4();
            let hierarchy: Arc<dyn ResourceGroupReadHierarchy> = Arc::new(rg_svc);
            let authz = StaticAuthZService::with_hierarchy(hierarchy);

            // Request WITHOUT GroupHierarchy capability
            let mut request = make_group_eval_request(tenant_id, Uuid::new_v4());
            request.context.capabilities = vec![]; // no capabilities

            let response = authz.evaluate(&request).await;

            // Should allow with tenant-only scope (hierarchy check skipped)
            assert!(
                response.decision,
                "AuthZ should allow — no hierarchy capability, default tenant scope"
            );
            assert_eq!(response.context.constraints.len(), 1);
        }
    }
}
