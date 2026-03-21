use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use modkit::api::OpenApiRegistry;
use modkit::{DatabaseCapability, Module, ModuleCtx, RestApiCapability};
use modkit_db::DBProvider;
use modkit_db::DbError;
use sea_orm_migration::MigrationTrait;
use tracing::info;

use crate::api::rest::routes;
use crate::domain::group_service::{GroupService, QueryProfile};
use crate::domain::membership_service::MembershipService;
use crate::domain::type_service::TypeService;

// @cpt-dod:cpt-cf-resource-group-dod-sdk-foundation-module-scaffold:p1
/// Main module struct for the resource-group module.
#[modkit::module(
    name = "resource-group",
    deps = [],
    capabilities = [db, rest]
)]
pub struct ResourceGroup {
    type_service: OnceLock<Arc<TypeService>>,
    group_service: OnceLock<Arc<GroupService>>,
    membership_service: OnceLock<Arc<MembershipService>>,
}

impl Default for ResourceGroup {
    fn default() -> Self {
        Self {
            type_service: OnceLock::new(),
            group_service: OnceLock::new(),
            membership_service: OnceLock::new(),
        }
    }
}

#[async_trait]
impl Module for ResourceGroup {
    async fn init(&self, ctx: &ModuleCtx) -> anyhow::Result<()> {
        // Acquire DB capability (secure wrapper)
        let db: Arc<DBProvider<DbError>> = Arc::new(ctx.db_required()?);

        // Create TypeService
        let type_service = Arc::new(TypeService::new(db.clone()));

        self.type_service
            .set(type_service)
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;

        // Create GroupService with default query profile
        let profile = QueryProfile::default();
        let group_service = Arc::new(GroupService::new(db.clone(), profile));

        self.group_service
            .set(group_service)
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;

        // Create MembershipService
        let membership_service = Arc::new(MembershipService::new(db));
        self.membership_service
            .set(membership_service)
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;

        info!("Resource Group module initialized");
        Ok(())
    }
}

impl DatabaseCapability for ResourceGroup {
    fn migrations(&self) -> Vec<Box<dyn MigrationTrait>> {
        use sea_orm_migration::MigratorTrait;
        info!("Providing resource_group database migrations");
        crate::infra::storage::migrations::Migrator::migrations()
    }
}

impl RestApiCapability for ResourceGroup {
    fn register_rest(
        &self,
        _ctx: &ModuleCtx,
        router: axum::Router,
        openapi: &dyn OpenApiRegistry,
    ) -> anyhow::Result<axum::Router> {
        info!("Registering resource_group REST routes");

        let type_service = self
            .type_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("TypeService not initialized"))?
            .clone();

        let group_service = self
            .group_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("GroupService not initialized"))?
            .clone();

        let membership_service = self
            .membership_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("MembershipService not initialized"))?
            .clone();

        let router = routes::register_routes(router, openapi, type_service, group_service, membership_service);

        info!("Resource Group REST routes registered successfully");
        Ok(router)
    }
}
