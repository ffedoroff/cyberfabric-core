use modkit_db::migration_runner::run_migrations_for_testing;
use modkit_db::outbox::{HandlerResult, OutboxMessage, TransactionalMessageHandler};
use sea_orm::{ConnectionTrait, DatabaseConnection, QueryResult};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::event::{ChangeType, TenantHierarchyChanged};
use super::handler::TenantClosureCdcHandler;

/// Helper: make an OutboxMessage from a CDC event.
fn make_msg(event: &TenantHierarchyChanged, seq: i64, attempts: i16) -> OutboxMessage {
    OutboxMessage {
        partition_id: 0,
        seq,
        payload: serde_json::to_vec(event).unwrap(),
        payload_type: "application/json;tenant_hierarchy_changed.v1".into(),
        created_at: chrono::Utc::now(),
        attempts,
    }
}

/// Helper: count rows in tenant_closure.
async fn count_closure(conn: &DatabaseConnection) -> i64 {
    let result: Option<QueryResult> = conn
        .query_one(sea_orm::Statement::from_string(
            conn.get_database_backend(),
            "SELECT COUNT(*) as cnt FROM tenant_closure".to_string(),
        ))
        .await
        .unwrap();
    result.unwrap().try_get_by_index::<i64>(0).unwrap()
}

/// Helper: query closure rows for a descendant.
async fn closure_ancestors(
    conn: &DatabaseConnection,
    descendant: Uuid,
) -> Vec<(String, String, i32, String)> {
    let results = conn
        .query_all(sea_orm::Statement::from_string(
            conn.get_database_backend(),
            format!(
                "SELECT hex(ancestor_id), hex(descendant_id), barrier, descendant_status \
                 FROM tenant_closure WHERE descendant_id = X'{}'",
                descendant.simple()
            ),
        ))
        .await
        .unwrap();

    results
        .iter()
        .map(|r| {
            (
                r.try_get_by_index::<String>(0).unwrap(),
                r.try_get_by_index::<String>(1).unwrap(),
                r.try_get_by_index::<i32>(2).unwrap(),
                r.try_get_by_index::<String>(3).unwrap(),
            )
        })
        .collect()
}

/// Helper: seed a parent tenant in tenant_closure (self-row only).
async fn seed_self_row(conn: &DatabaseConnection, id: Uuid, status: &str) {
    conn.execute(sea_orm::Statement::from_string(
        conn.get_database_backend(),
        format!(
            "INSERT INTO tenant_closure (ancestor_id, descendant_id, barrier, descendant_status) \
             VALUES (X'{simple}', X'{simple}', 0, '{status}')",
            simple = id.simple(),
        ),
    ))
    .await
    .unwrap();
}

#[tokio::test]
async fn cdc_created_inserts_self_row() {
    let (_, conn) = setup_test_db().await;
    let handler = TenantClosureCdcHandler;

    let tenant = Uuid::new_v4();
    let event = TenantHierarchyChanged::new(tenant, ChangeType::Created, "active");
    let msg = make_msg(&event, 1, 0);

    let result = handler
        .handle(&conn, &msg, CancellationToken::new())
        .await;
    assert!(matches!(result, HandlerResult::Success));

    let count = count_closure(&conn).await;
    assert_eq!(count, 1, "Should have 1 self-row");

    let rows = closure_ancestors(&conn, tenant).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].2, 0, "Self-row barrier should be 0");
    assert_eq!(rows[0].3, "active");
}

#[tokio::test]
async fn cdc_created_with_parent_inherits_ancestors() {
    let (_, conn) = setup_test_db().await;
    let handler = TenantClosureCdcHandler;

    // Seed root tenant
    let root = Uuid::new_v4();
    seed_self_row(&conn, root, "active").await;

    // Create child under root
    let child = Uuid::new_v4();
    let event = TenantHierarchyChanged::new(child, ChangeType::Created, "active")
        .with_ancestor(root);
    let msg = make_msg(&event, 1, 0);

    let result = handler
        .handle(&conn, &msg, CancellationToken::new())
        .await;
    assert!(matches!(result, HandlerResult::Success));

    // child should have 2 rows: self-row + ancestor row to root
    let rows = closure_ancestors(&conn, child).await;
    assert_eq!(rows.len(), 2, "Should have self-row + 1 ancestor row");
}

#[tokio::test]
async fn cdc_deleted_removes_all_rows() {
    let (_, conn) = setup_test_db().await;
    let handler = TenantClosureCdcHandler;

    let tenant = Uuid::new_v4();

    // Create tenant
    let create_event = TenantHierarchyChanged::new(tenant, ChangeType::Created, "active");
    handler
        .handle(&conn, &make_msg(&create_event, 1, 0), CancellationToken::new())
        .await;
    assert_eq!(count_closure(&conn).await, 1);

    // Delete tenant
    let delete_event = TenantHierarchyChanged::new(tenant, ChangeType::Deleted, "deleted");
    let result = handler
        .handle(&conn, &make_msg(&delete_event, 2, 0), CancellationToken::new())
        .await;
    assert!(matches!(result, HandlerResult::Success));
    assert_eq!(count_closure(&conn).await, 0, "All rows should be deleted");
}

#[tokio::test]
async fn cdc_status_changed_updates_descendant_status() {
    let (_, conn) = setup_test_db().await;
    let handler = TenantClosureCdcHandler;

    let tenant = Uuid::new_v4();

    // Create tenant
    let create_event = TenantHierarchyChanged::new(tenant, ChangeType::Created, "active");
    handler
        .handle(&conn, &make_msg(&create_event, 1, 0), CancellationToken::new())
        .await;

    // Change status
    let status_event = TenantHierarchyChanged::new(tenant, ChangeType::StatusChanged, "suspended");
    let result = handler
        .handle(&conn, &make_msg(&status_event, 2, 0), CancellationToken::new())
        .await;
    assert!(matches!(result, HandlerResult::Success));

    let rows = closure_ancestors(&conn, tenant).await;
    assert_eq!(rows[0].3, "suspended", "Status should be updated");
}

#[tokio::test]
async fn cdc_invalid_payload_is_rejected() {
    let (_, conn) = setup_test_db().await;
    let handler = TenantClosureCdcHandler;

    let msg = OutboxMessage {
        partition_id: 0,
        seq: 1,
        payload: b"invalid json".to_vec(),
        payload_type: "text/plain".into(),
        created_at: chrono::Utc::now(),
        attempts: 0,
    };

    let result = handler
        .handle(&conn, &msg, CancellationToken::new())
        .await;
    assert!(
        matches!(result, HandlerResult::Reject { .. }),
        "Invalid payload should be rejected"
    );
}

#[tokio::test]
async fn cdc_created_with_barrier_propagates() {
    let (_, conn) = setup_test_db().await;
    let handler = TenantClosureCdcHandler;

    // Seed root
    let root = Uuid::new_v4();
    seed_self_row(&conn, root, "active").await;

    // Create child with barrier
    let child = Uuid::new_v4();
    let event = TenantHierarchyChanged::new(child, ChangeType::Created, "active")
        .with_ancestor(root)
        .with_barrier(1);
    let msg = make_msg(&event, 1, 0);

    handler
        .handle(&conn, &msg, CancellationToken::new())
        .await;

    let rows = closure_ancestors(&conn, child).await;
    // The ancestor row (root → child) should have barrier = 1
    let ancestor_row = rows.iter().find(|(a, d, _, _)| a != d).unwrap();
    assert_eq!(ancestor_row.2, 1, "Barrier should be propagated");
}

/// Setup test database with resource-group migrations (includes tenant_closure).
/// Returns both the Db (to keep it alive) and the raw DatabaseConnection.
async fn setup_test_db() -> (modkit_db::Db, DatabaseConnection) {
    use sea_orm_migration::MigratorTrait;
    let db = modkit_db::connect_db(
        "sqlite::memory:",
        modkit_db::ConnectOpts {
            max_conns: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    run_migrations_for_testing(
        &db,
        crate::infra::db::migrations::Migrator::migrations(),
    )
    .await
    .unwrap();

    let conn = db.raw_conn_for_testing();
    (db, conn)
}
