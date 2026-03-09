//! Integration tests for the Resource Group service.
//!
//! These tests use an in-memory `SQLite` database since `DBRunner` is a sealed trait
//! and cannot be mocked. All tests use real database operations via `RgService` facade.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use modkit_db::migration_runner::run_migrations_for_testing;
    use modkit_db::{ConnectOpts, DBProvider, Db, connect_db};
    use resource_group_sdk::{
        AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery,
        RemoveMembershipRequest, UpdateGroupRequest, UpdateTypeRequest,
    };
    use uuid::Uuid;

    use crate::domain::error::DomainError;
    use crate::domain::service::RgService;
    use crate::infra::db::migrations::Migrator;

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
        RgService::new(db, None, None)
    }

    fn build_service_with_limits(db: Db, max_depth: usize, max_width: usize) -> RgService {
        let db: Arc<DBProvider<modkit_db::DbError>> = Arc::new(DBProvider::new(db));
        RgService::new(db, Some(max_depth), Some(max_width))
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
        let g = svc
            .group_service()
            .create_group(CreateGroupRequest {
                group_type: "tenant".into(),
                name: name.into(),
                parent_id: None,
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
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
            .create_group(CreateGroupRequest {
                group_type: "tenant".into(),
                name: "Acme Corp".into(),
                parent_id: None,
                tenant_id,
                external_id: Some("ext-1".into()),
            })
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
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Engineering".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
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
            .create_group(CreateGroupRequest {
                group_type: "tenant".into(),
                name: "Bad".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidParentType { .. }));
    }

    #[tokio::test]
    async fn group_create_type_not_found() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .group_service()
            .create_group(CreateGroupRequest {
                group_type: "nonexistent".into(),
                name: "Bad".into(),
                parent_id: None,
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TypeNotFound { .. }));
    }

    #[tokio::test]
    async fn group_get_and_not_found() {
        let svc = build_service(inmem_db().await);
        seed_tenant_type(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        let group = svc.group_service().get_group(root_id).await.unwrap();
        assert_eq!(group.group_id, root_id);

        let err = svc
            .group_service()
            .get_group(Uuid::new_v4())
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
            .list_groups(ListQuery::default())
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
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root1),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
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
            .create_group(CreateGroupRequest {
                group_type: "node".into(),
                name: "Root".into(),
                parent_id: None,
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap();

        let child = svc
            .group_service()
            .create_group(CreateGroupRequest {
                group_type: "node".into(),
                name: "Child".into(),
                parent_id: Some(root.group_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
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
            .delete_group(root_id, false)
            .await
            .unwrap();

        let err = svc
            .group_service()
            .get_group(root_id)
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
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap();

        let err = svc
            .group_service()
            .delete_group(root_id, false)
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
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap();

        // Force delete should cascade
        svc.group_service()
            .delete_group(root_id, true)
            .await
            .unwrap();

        assert!(svc.group_service().get_group(root_id).await.is_err());
        assert!(svc.group_service().get_group(child.group_id).await.is_err());
    }

    #[tokio::test]
    async fn group_depth_traversal() {
        let svc = build_service(inmem_db().await);
        seed_types(&svc).await;

        let root_id = create_root_group(&svc, "Root").await;

        let child = svc
            .group_service()
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Child".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap();

        // Depth from root: root=0, child=1
        let page = svc
            .group_service()
            .list_group_depth(root_id, ListQuery::default())
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
            .list_group_depth(child.group_id, ListQuery::default())
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
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Org".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap();

        // max_depth=1, child is at depth 1 already, grandchild would be depth 2 — blocked
        let err = svc
            .group_service()
            .create_group(CreateGroupRequest {
                group_type: "dept".into(),
                name: "Dept".into(),
                parent_id: Some(child.group_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
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
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Org1".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
            .await
            .unwrap();

        // max_width=1, root already has 1 child — second child blocked
        let err = svc
            .group_service()
            .create_group(CreateGroupRequest {
                group_type: "org".into(),
                name: "Org2".into(),
                parent_id: Some(root_id),
                tenant_id: Uuid::new_v4(),
                external_id: None,
            })
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
            .add_membership(AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap();

        assert_eq!(mbr.group_id, group_id);
        assert_eq!(mbr.resource_type, "user");
        assert_eq!(mbr.resource_id, "u-1");

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default())
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
            .add_membership(AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap();

        let err = svc
            .membership_service()
            .add_membership(AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::ActiveReferences { .. }));
    }

    #[tokio::test]
    async fn membership_add_group_not_found() {
        let svc = build_service(inmem_db().await);

        let err = svc
            .membership_service()
            .add_membership(AddMembershipRequest {
                group_id: Uuid::new_v4(),
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
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
            .add_membership(AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap();

        svc.membership_service()
            .remove_membership(RemoveMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap();

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default())
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
            .remove_membership(RemoveMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
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
            .list_memberships(ListQuery::default())
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
            .add_membership(AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap();

        let err = svc
            .group_service()
            .delete_group(group_id, false)
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
            .add_membership(AddMembershipRequest {
                group_id,
                resource_type: "user".into(),
                resource_id: "u-1".into(),
            })
            .await
            .unwrap();

        // Force delete should cascade memberships too
        svc.group_service()
            .delete_group(group_id, true)
            .await
            .unwrap();

        assert!(svc.group_service().get_group(group_id).await.is_err());

        let page = svc
            .membership_service()
            .list_memberships(ListQuery::default())
            .await
            .unwrap();
        assert_eq!(page.items.len(), 0);
    }
}
