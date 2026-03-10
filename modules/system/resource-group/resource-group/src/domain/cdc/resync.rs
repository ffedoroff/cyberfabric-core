// @cpt-req:cpt-cf-resource-group-dod-full-resync:p2
// @cpt-flow:cpt-cf-resource-group-flow-tenant-closure-seed:p2

use sea_orm::{ConnectionTrait, ExecResult};
use std::sync::Arc;
use tenant_resolver_sdk::{
    BarrierMode, GetDescendantsOptions, TenantResolverClient, TenantResolverError, TenantStatus,
};
use uuid::Uuid;

/// Performs a full resync of the `tenant_closure` table by reading the complete
/// tenant hierarchy from the tenant-resolver module via SDK.
///
/// This is used for:
/// - Initial seed on first deployment (empty table)
/// - Recovery after data corruption
/// - Operator-triggered manual resync
pub struct TenantClosureResyncer {
    tenant_client: Arc<dyn TenantResolverClient>,
}

impl TenantClosureResyncer {
    pub fn new(tenant_client: Arc<dyn TenantResolverClient>) -> Self {
        Self { tenant_client }
    }

    /// Perform a full resync: truncate and rebuild `tenant_closure` from
    /// the tenant-resolver's current hierarchy state.
    ///
    /// This reads the full descendant tree from a root tenant and computes
    /// the transitive closure.
    // @cpt-begin:cpt-cf-resource-group-flow-tenant-closure-seed:p2:inst-seed-2
    pub async fn resync(
        &self,
        txn: &dyn ConnectionTrait,
        root_tenant_id: Uuid,
        ctx: &modkit_security::SecurityContext,
    ) -> Result<u64, ResyncError> {
        tracing::info!(root_tenant_id = %root_tenant_id, "Starting tenant_closure full resync");

        // Step 1: Read full hierarchy from tenant-resolver
        let options = GetDescendantsOptions {
            status: vec![TenantStatus::Active, TenantStatus::Suspended],
            barrier_mode: BarrierMode::Ignore,
            max_depth: None,
        };
        let response = self
            .tenant_client
            .get_descendants(ctx, root_tenant_id, &options)
            .await
            .map_err(ResyncError::TenantResolver)?;

        // Step 2: Truncate existing data
        exec_sql(txn, "DELETE FROM tenant_closure").await?;

        // Step 3: Build closure rows from hierarchy
        // The root tenant itself + all descendants
        let mut row_count: u64 = 0;

        // Self-row for root
        let root = &response.tenant;
        insert_self_row(txn, root.id, &status_str(root.status)).await?;
        row_count += 1;

        // Build parent map for closure computation
        let mut tenants: Vec<(Uuid, Option<Uuid>, bool, String)> = Vec::new();
        tenants.push((root.id, None, root.self_managed, status_str(root.status)));
        for desc in &response.descendants {
            tenants.push((
                desc.id,
                desc.parent_id,
                desc.self_managed,
                status_str(desc.status),
            ));
        }

        // For each descendant, insert self-row + closure rows to all ancestors
        for desc in &response.descendants {
            // Self-row
            insert_self_row(txn, desc.id, &status_str(desc.status)).await?;
            row_count += 1;

            // Walk up parent chain and insert closure rows
            let mut current_id = desc.parent_id;
            let mut barrier_accumulated = if desc.self_managed { 1 } else { 0 };

            while let Some(parent_id) = current_id {
                let barrier_val = barrier_accumulated;
                let status = &status_str(desc.status);

                exec_sql(
                    txn,
                    &format!(
                        "INSERT OR IGNORE INTO tenant_closure \
                         (ancestor_id, descendant_id, barrier, descendant_status) \
                         VALUES ({anc}, {desc_id}, {barrier_val}, '{status}')",
                        anc = blob_literal(parent_id),
                        desc_id = blob_literal(desc.id),
                    ),
                )
                .await?;
                row_count += 1;

                // Find parent in our tenants list to continue walking up
                if let Some((_, grandparent, is_barrier, _)) = tenants
                    .iter()
                    .find(|(id, _, _, _)| *id == parent_id)
                {
                    if *is_barrier {
                        barrier_accumulated = 1;
                    }
                    current_id = *grandparent;
                } else {
                    break;
                }
            }
        }

        tracing::info!(
            root_tenant_id = %root_tenant_id,
            row_count,
            tenant_count = tenants.len(),
            "tenant_closure full resync completed"
        );

        Ok(row_count)
    }
    // @cpt-end:cpt-cf-resource-group-flow-tenant-closure-seed:p2:inst-seed-2

    /// Check if the `tenant_closure` table is empty (needs initial seed).
    pub async fn is_empty(&self, conn: &dyn ConnectionTrait) -> Result<bool, ResyncError> {
        let result = conn
            .query_one(sea_orm::Statement::from_string(
                conn.get_database_backend(),
                "SELECT COUNT(*) as cnt FROM tenant_closure".to_string(),
            ))
            .await
            .map_err(ResyncError::Db)?;

        match result {
            Some(row) => {
                let count: i64 = row.try_get_by_index(0).unwrap_or(0);
                Ok(count == 0)
            }
            None => Ok(true),
        }
    }
}

async fn insert_self_row(
    txn: &dyn ConnectionTrait,
    id: Uuid,
    status: &str,
) -> Result<ExecResult, sea_orm::DbErr> {
    exec_sql(
        txn,
        &format!(
            "INSERT OR IGNORE INTO tenant_closure \
             (ancestor_id, descendant_id, barrier, descendant_status) \
             VALUES ({id_lit}, {id_lit}, 0, '{status}')",
            id_lit = blob_literal(id),
        ),
    )
    .await
}

fn blob_literal(id: Uuid) -> String {
    format!("X'{}'", id.simple())
}

fn status_str(status: TenantStatus) -> String {
    match status {
        TenantStatus::Active => "active".to_string(),
        TenantStatus::Suspended => "suspended".to_string(),
        TenantStatus::Deleted => "deleted".to_string(),
    }
}

async fn exec_sql(
    txn: &dyn ConnectionTrait,
    sql: &str,
) -> Result<ExecResult, sea_orm::DbErr> {
    txn.execute(sea_orm::Statement::from_string(
        txn.get_database_backend(),
        sql.to_string(),
    ))
    .await
}

/// Errors during tenant_closure resync.
#[derive(Debug, thiserror::Error)]
pub enum ResyncError {
    #[error("tenant-resolver error: {0}")]
    TenantResolver(TenantResolverError),
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
}
