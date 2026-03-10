// @cpt-req:cpt-cf-resource-group-dod-cdc-consumer:p2
// @cpt-algo:cpt-cf-resource-group-algo-closure-delta:p2

use async_trait::async_trait;
use modkit_db::outbox::{HandlerResult, OutboxMessage, TransactionalMessageHandler};
use sea_orm::{ConnectionTrait, ExecResult};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::event::{ChangeType, TenantHierarchyChanged};

/// CDC consumer that maintains the `tenant_closure` local projection table.
///
/// Implements `TransactionalMessageHandler` for exactly-once delivery:
/// projection updates and outbox cursor advance happen atomically.
pub struct TenantClosureCdcHandler;

#[async_trait]
impl TransactionalMessageHandler for TenantClosureCdcHandler {
    // @cpt-begin:cpt-cf-resource-group-flow-tenant-closure-cdc:p2:inst-cdc-5
    async fn handle(
        &self,
        txn: &dyn ConnectionTrait,
        msg: &OutboxMessage,
        _cancel: CancellationToken,
    ) -> HandlerResult {
        let event = match serde_json::from_slice::<TenantHierarchyChanged>(&msg.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(error = %e, seq = msg.seq, "Failed to parse TenantHierarchyChanged event");
                return HandlerResult::Reject {
                    reason: format!("invalid CDC payload: {e}"),
                };
            }
        };

        tracing::debug!(
            tenant_id = %event.tenant_id,
            change_type = ?event.change_type,
            seq = msg.seq,
            "Processing tenant hierarchy CDC event"
        );

        // @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-1
        match apply_closure_delta(txn, &event).await {
            Ok(()) => {
                tracing::debug!(
                    tenant_id = %event.tenant_id,
                    change_type = ?event.change_type,
                    "Closure delta applied successfully"
                );
                HandlerResult::Success
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    tenant_id = %event.tenant_id,
                    change_type = ?event.change_type,
                    attempts = msg.attempts,
                    "Failed to apply closure delta"
                );
                if msg.attempts >= 3 {
                    HandlerResult::Reject {
                        reason: format!("persistent failure after {} attempts: {e}", msg.attempts + 1),
                    }
                } else {
                    HandlerResult::Retry {
                        reason: format!("closure delta failed: {e}"),
                    }
                }
            }
        }
        // @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-1
    }
    // @cpt-end:cpt-cf-resource-group-flow-tenant-closure-cdc:p2:inst-cdc-5
}

/// Apply the closure delta for a given CDC event.
///
/// Each change type produces a different set of INSERT/DELETE/UPDATE
/// operations on the `tenant_closure` table.
// @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-2
async fn apply_closure_delta(
    txn: &dyn ConnectionTrait,
    event: &TenantHierarchyChanged,
) -> Result<(), sea_orm::DbErr> {
    match event.change_type {
        ChangeType::Created => apply_created(txn, event).await,
        ChangeType::Deleted => apply_deleted(txn, event).await,
        ChangeType::Moved => apply_moved(txn, event).await,
        ChangeType::StatusChanged => apply_status_changed(txn, event).await,
        ChangeType::BarrierChanged => apply_barrier_changed(txn, event).await,
    }
}
// @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-2

/// Created: insert self-row + rows from all ancestors to this tenant.
// @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-1a
async fn apply_created(
    txn: &dyn ConnectionTrait,
    event: &TenantHierarchyChanged,
) -> Result<(), sea_orm::DbErr> {
    let tid = event.tenant_id;
    let status = &event.status;

    // Self-row: (tenant, tenant, 0, status)
    exec_sql(
        txn,
        &format!(
            "INSERT OR IGNORE INTO tenant_closure (ancestor_id, descendant_id, barrier, descendant_status) \
             VALUES ({anc}, {desc}, 0, '{status}')",
            anc = blob_literal(tid),
            desc = blob_literal(tid),
        ),
    )
    .await?;

    // If tenant has a parent, inherit all ancestor rows:
    // For each existing ancestor of the parent, create a row (ancestor, new_tenant, barrier, status)
    if let Some(parent_id) = event.ancestor_id {
        exec_sql(
            txn,
            &format!(
                "INSERT OR IGNORE INTO tenant_closure (ancestor_id, descendant_id, barrier, descendant_status) \
                 SELECT tc.ancestor_id, {desc}, \
                        CASE WHEN tc.barrier > 0 OR {barrier} > 0 THEN 1 ELSE 0 END, \
                        '{status}' \
                 FROM tenant_closure tc \
                 WHERE tc.descendant_id = {parent}",
                desc = blob_literal(tid),
                parent = blob_literal(parent_id),
                barrier = event.barrier,
            ),
        )
        .await?;
    }

    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-1a

/// Deleted: remove all closure rows referencing this tenant.
// @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-2a
async fn apply_deleted(
    txn: &dyn ConnectionTrait,
    event: &TenantHierarchyChanged,
) -> Result<(), sea_orm::DbErr> {
    let tid = event.tenant_id;

    // Delete all rows where this tenant is a descendant
    exec_sql(
        txn,
        &format!(
            "DELETE FROM tenant_closure WHERE descendant_id = {id}",
            id = blob_literal(tid),
        ),
    )
    .await?;

    // Delete all rows where this tenant is an ancestor (subtree entries for children)
    exec_sql(
        txn,
        &format!(
            "DELETE FROM tenant_closure WHERE ancestor_id = {id}",
            id = blob_literal(tid),
        ),
    )
    .await?;

    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-2a

/// Moved: remove old ancestor path, recompute new ancestor path.
// @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-3a
async fn apply_moved(
    txn: &dyn ConnectionTrait,
    event: &TenantHierarchyChanged,
) -> Result<(), sea_orm::DbErr> {
    let tid = event.tenant_id;

    // Collect all descendants of the moved tenant (including self)
    // Then delete all ancestor rows for this subtree that come from the OLD path
    // and re-insert from the NEW parent path.

    // Step 1: Delete closure entries from old ancestor path for the entire subtree
    // (Keep self-rows and internal subtree relationships)
    exec_sql(
        txn,
        &format!(
            "DELETE FROM tenant_closure \
             WHERE descendant_id IN (SELECT descendant_id FROM tenant_closure WHERE ancestor_id = {id}) \
             AND ancestor_id NOT IN (SELECT descendant_id FROM tenant_closure WHERE ancestor_id = {id})",
            id = blob_literal(tid),
        ),
    )
    .await?;

    // Step 2: Re-insert closure entries from new parent path
    if let Some(new_parent_id) = event.ancestor_id {
        // For each ancestor of the new parent, and each descendant in our subtree,
        // create a closure row
        exec_sql(
            txn,
            &format!(
                "INSERT OR IGNORE INTO tenant_closure (ancestor_id, descendant_id, barrier, descendant_status) \
                 SELECT pa.ancestor_id, sub.descendant_id, \
                        CASE WHEN pa.barrier > 0 OR sub.barrier > 0 THEN 1 ELSE 0 END, \
                        sub.descendant_status \
                 FROM tenant_closure pa \
                 CROSS JOIN tenant_closure sub \
                 WHERE pa.descendant_id = {new_parent} \
                   AND sub.ancestor_id = {id}",
                new_parent = blob_literal(new_parent_id),
                id = blob_literal(tid),
            ),
        )
        .await?;
    }

    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-3a

/// Status changed: update descendant_status for all rows where descendant = tenant.
// @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-4a
async fn apply_status_changed(
    txn: &dyn ConnectionTrait,
    event: &TenantHierarchyChanged,
) -> Result<(), sea_orm::DbErr> {
    let tid = event.tenant_id;
    let status = &event.status;

    exec_sql(
        txn,
        &format!(
            "UPDATE tenant_closure SET descendant_status = '{status}' \
             WHERE descendant_id = {id}",
            id = blob_literal(tid),
        ),
    )
    .await?;

    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-4a

/// Barrier changed: recompute barrier values for affected subtree entries.
// @cpt-begin:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-5a
async fn apply_barrier_changed(
    txn: &dyn ConnectionTrait,
    event: &TenantHierarchyChanged,
) -> Result<(), sea_orm::DbErr> {
    let tid = event.tenant_id;
    let barrier = event.barrier;

    // Update the barrier for all rows where ancestor passes through this tenant.
    // Rows where descendant is "below" this tenant in the hierarchy and ancestor is
    // "above" it should reflect the new barrier value.
    //
    // Simplified: set barrier=1 on all rows where the path passes through a
    // self_managed tenant, barrier=0 otherwise. For single barrier change,
    // we update rows involving this tenant.
    exec_sql(
        txn,
        &format!(
            "UPDATE tenant_closure SET barrier = {barrier} \
             WHERE descendant_id IN (SELECT descendant_id FROM tenant_closure WHERE ancestor_id = {id}) \
             AND ancestor_id NOT IN (SELECT descendant_id FROM tenant_closure WHERE ancestor_id = {id})",
            id = blob_literal(tid),
        ),
    )
    .await?;

    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-closure-delta:p2:inst-delta-5a

/// Format UUID as SQLite BLOB hex literal: `X'<32 hex chars>'`.
///
/// This matches SeaORM's storage format for UUID columns in SQLite.
/// See INTEGRATION_AUTHZ.md C8 for rationale.
fn blob_literal(id: Uuid) -> String {
    format!("X'{}'", id.simple())
}

async fn exec_sql(txn: &dyn ConnectionTrait, sql: &str) -> Result<ExecResult, sea_orm::DbErr> {
    txn.execute(sea_orm::Statement::from_string(
        txn.get_database_backend(),
        sql.to_string(),
    ))
    .await
}
