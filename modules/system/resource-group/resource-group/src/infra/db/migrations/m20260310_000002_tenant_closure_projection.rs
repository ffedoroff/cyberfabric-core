// @cpt-req:cpt-cf-resource-group-dod-db-migration:p1

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// Local projection of `tenant_closure` table.
///
/// Owned by tenant-resolver module; this is a **read-only projection** populated
/// via CDC (Change Data Capture) from the tenant-resolver event stream.
///
/// Required for `InTenantSubtree` predicate SQL:
/// ```sql
/// SELECT descendant_id FROM tenant_closure
///   WHERE ancestor_id = ?
///   [AND barrier = 0]
///   [AND descendant_status IN (?)]
/// ```
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    #[allow(elided_lifetimes_in_paths)]
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let sql = match backend {
            sea_orm::DatabaseBackend::Postgres => POSTGRES_UP,
            sea_orm::DatabaseBackend::Sqlite => SQLITE_UP,
            sea_orm::DatabaseBackend::MySql => {
                return Err(DbErr::Migration("MySQL not supported".into()));
            }
        };
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    #[allow(elided_lifetimes_in_paths)]
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}

const POSTGRES_UP: &str = r"
CREATE TABLE tenant_closure (
    ancestor_id UUID NOT NULL,
    descendant_id UUID NOT NULL,
    barrier INT NOT NULL DEFAULT 0,
    descendant_status TEXT NOT NULL DEFAULT 'active',
    PRIMARY KEY (ancestor_id, descendant_id)
);

CREATE INDEX idx_tc_descendant_id ON tenant_closure (descendant_id);
CREATE INDEX idx_tc_ancestor_barrier ON tenant_closure (ancestor_id, barrier);
CREATE INDEX idx_tc_ancestor_status ON tenant_closure (ancestor_id, descendant_status);
";

const SQLITE_UP: &str = r"
CREATE TABLE tenant_closure (
    ancestor_id TEXT NOT NULL,
    descendant_id TEXT NOT NULL,
    barrier INTEGER NOT NULL DEFAULT 0,
    descendant_status TEXT NOT NULL DEFAULT 'active',
    PRIMARY KEY (ancestor_id, descendant_id)
);

CREATE INDEX idx_tc_descendant_id ON tenant_closure (descendant_id);
CREATE INDEX idx_tc_ancestor_barrier ON tenant_closure (ancestor_id, barrier);
CREATE INDEX idx_tc_ancestor_status ON tenant_closure (ancestor_id, descendant_status);
";

const DOWN: &str = r"
DROP INDEX IF EXISTS idx_tc_ancestor_status;
DROP INDEX IF EXISTS idx_tc_ancestor_barrier;
DROP INDEX IF EXISTS idx_tc_descendant_id;
DROP TABLE IF EXISTS tenant_closure;
";
