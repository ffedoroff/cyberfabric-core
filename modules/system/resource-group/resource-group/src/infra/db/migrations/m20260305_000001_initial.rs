use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

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
CREATE TABLE resource_group_type (
    code TEXT PRIMARY KEY,
    parents TEXT[] NOT NULL CHECK (cardinality(parents) >= 1),
    created TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified TIMESTAMPTZ DEFAULT NULL
);

CREATE UNIQUE INDEX idx_resource_group_type_code_lower
    ON resource_group_type (LOWER(code));

CREATE TABLE resource_group (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID,
    group_type TEXT NOT NULL,
    name TEXT NOT NULL,
    tenant_id UUID NOT NULL,
    external_id TEXT,
    created TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified TIMESTAMPTZ DEFAULT NULL,
    CONSTRAINT fk_resource_group_type
        FOREIGN KEY (group_type)
        REFERENCES resource_group_type(code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_resource_group_parent
        FOREIGN KEY (parent_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX idx_rg_parent_id ON resource_group (parent_id);
CREATE INDEX idx_rg_name ON resource_group (name);
CREATE INDEX idx_rg_external_id ON resource_group (external_id);
CREATE INDEX idx_rg_group_type ON resource_group (group_type, id);

CREATE TABLE resource_group_closure (
    ancestor_id UUID NOT NULL,
    descendant_id UUID NOT NULL,
    depth INTEGER NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id),
    CONSTRAINT fk_closure_ancestor
        FOREIGN KEY (ancestor_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_closure_descendant
        FOREIGN KEY (descendant_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX idx_rgc_descendant_id ON resource_group_closure (descendant_id);
CREATE INDEX idx_rgc_ancestor_depth ON resource_group_closure (ancestor_id, depth);

CREATE TABLE resource_group_membership (
    group_id UUID NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    created TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_resource_group_membership_group_id
        FOREIGN KEY (group_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_membership_unique
        UNIQUE (group_id, resource_type, resource_id)
);

CREATE INDEX idx_rgm_resource_type_id
    ON resource_group_membership (resource_type, resource_id);
";

const SQLITE_UP: &str = r"
CREATE TABLE resource_group_type (
    code TEXT PRIMARY KEY NOT NULL,
    parents TEXT NOT NULL DEFAULT '[]',
    created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    modified TEXT DEFAULT NULL
);

CREATE UNIQUE INDEX idx_resource_group_type_code_lower
    ON resource_group_type (code COLLATE NOCASE);

CREATE TABLE resource_group (
    id TEXT PRIMARY KEY NOT NULL,
    parent_id TEXT,
    group_type TEXT NOT NULL,
    name TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    external_id TEXT,
    created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    modified TEXT DEFAULT NULL,
    CONSTRAINT fk_resource_group_type
        FOREIGN KEY (group_type)
        REFERENCES resource_group_type(code)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_resource_group_parent
        FOREIGN KEY (parent_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX idx_rg_parent_id ON resource_group (parent_id);
CREATE INDEX idx_rg_name ON resource_group (name);
CREATE INDEX idx_rg_external_id ON resource_group (external_id);
CREATE INDEX idx_rg_group_type ON resource_group (group_type, id);

CREATE TABLE resource_group_closure (
    ancestor_id TEXT NOT NULL,
    descendant_id TEXT NOT NULL,
    depth INTEGER NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id),
    CONSTRAINT fk_closure_ancestor
        FOREIGN KEY (ancestor_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT fk_closure_descendant
        FOREIGN KEY (descendant_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

CREATE INDEX idx_rgc_descendant_id ON resource_group_closure (descendant_id);
CREATE INDEX idx_rgc_ancestor_depth ON resource_group_closure (ancestor_id, depth);

CREATE TABLE resource_group_membership (
    group_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CONSTRAINT fk_resource_group_membership_group_id
        FOREIGN KEY (group_id)
        REFERENCES resource_group(id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT uq_resource_group_membership_unique
        UNIQUE (group_id, resource_type, resource_id)
);

CREATE INDEX idx_rgm_resource_type_id
    ON resource_group_membership (resource_type, resource_id);
";

const DOWN: &str = r"
DROP TABLE IF EXISTS resource_group_membership;
DROP TABLE IF EXISTS resource_group_closure;
DROP TABLE IF EXISTS resource_group;
DROP TABLE IF EXISTS resource_group_type;
";
