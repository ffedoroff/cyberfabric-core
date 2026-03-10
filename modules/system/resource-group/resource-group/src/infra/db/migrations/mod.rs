use sea_orm_migration::prelude::*;

mod m20260305_000001_initial;
mod m20260310_000002_tenant_closure_projection;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260305_000001_initial::Migration),
            Box::new(m20260310_000002_tenant_closure_projection::Migration),
        ]
    }
}
