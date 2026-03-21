//! Database migrations for the resource-group module.

use sea_orm_migration::MigratorTrait;

mod m20260306_000001_initial;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        vec![Box::new(m20260306_000001_initial::Migration)]
    }
}
