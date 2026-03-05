use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use modkit::api::OpenApiRegistry;
use modkit::{DatabaseCapability, Module, ModuleCtx, RestApiCapability};
use resource_group_sdk::{ResourceGroupClient, ResourceGroupReadHierarchy};
use sea_orm_migration::MigrationTrait;
use tracing::info;

use crate::api::rest::routes;
use crate::config::ResourceGroupConfig;
use crate::domain::service::RgService;

/// Resource Group module: hierarchical group management with closure-table topology.
///
/// Phase 1 (SystemCapability): registers `ResourceGroupClient` and
/// `ResourceGroupReadHierarchy` in `ClientHub`. REST endpoints NOT yet accepting traffic.
///
/// Phase 2 (ready): REST endpoints start accepting traffic after `AuthZ` Resolver
/// has completed its init.
#[modkit::module(
    name = "resource-group",
    capabilities = [db, rest],
)]
pub struct ResourceGroupModule {
    service: OnceLock<Arc<RgService>>,
    url_prefix: OnceLock<String>,
}

impl Default for ResourceGroupModule {
    fn default() -> Self {
        Self {
            service: OnceLock::new(),
            url_prefix: OnceLock::new(),
        }
    }
}

#[async_trait]
impl Module for ResourceGroupModule {
    async fn init(&self, ctx: &ModuleCtx) -> anyhow::Result<()> {
        info!("Initializing {} module", Self::MODULE_NAME);

        let cfg: ResourceGroupConfig = ctx.config()?;

        self.url_prefix
            .set(cfg.url_prefix)
            .map_err(|_| anyhow::anyhow!("{} url_prefix already set", Self::MODULE_NAME))?;

        let db = Arc::new(ctx.db_required()?);

        // Create unified service facade
        let svc = Arc::new(RgService::new(db));

        // Phase 1: Register SDK clients in ClientHub
        let client: Arc<dyn ResourceGroupClient> = svc.clone();
        ctx.client_hub().register::<dyn ResourceGroupClient>(client);

        let read_hierarchy: Arc<dyn ResourceGroupReadHierarchy> = svc.clone();
        ctx.client_hub()
            .register::<dyn ResourceGroupReadHierarchy>(read_hierarchy);

        self.service
            .set(svc)
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;

        info!(
            "{} module initialized \u{2014} SDK clients registered in ClientHub",
            Self::MODULE_NAME
        );
        Ok(())
    }
}

impl DatabaseCapability for ResourceGroupModule {
    fn migrations(&self) -> Vec<Box<dyn MigrationTrait>> {
        use sea_orm_migration::MigratorTrait;
        info!("Providing resource-group database migrations");
        crate::infra::db::migrations::Migrator::migrations()
    }
}

impl RestApiCapability for ResourceGroupModule {
    fn register_rest(
        &self,
        _ctx: &ModuleCtx,
        router: axum::Router,
        openapi: &dyn OpenApiRegistry,
    ) -> anyhow::Result<axum::Router> {
        let service = self
            .service
            .get()
            .ok_or_else(|| anyhow::anyhow!("{} not initialized", Self::MODULE_NAME))?;

        let prefix = self
            .url_prefix
            .get()
            .ok_or_else(|| anyhow::anyhow!("{} url_prefix not set", Self::MODULE_NAME))?;

        info!("Registering resource-group REST routes under {prefix}/v1");
        let router = routes::register_routes(router, openapi, Arc::clone(service), prefix);
        info!("Resource-group REST routes registered");
        Ok(router)
    }
}
