// @cpt-req:cpt-cf-resource-group-dod-module-lifecycle:p1
// @cpt-req:cpt-cf-resource-group-dod-init-order:p1
// @cpt-flow:cpt-cf-resource-group-flow-module-bootstrap:p1
// @cpt-algo:cpt-cf-resource-group-algo-phased-init:p1

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use authz_resolver_sdk::{AuthZResolverClient, Capability, PolicyEnforcer};
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
    deps = ["authz-resolver"],
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
        // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-1
        info!("Initializing {} module", Self::MODULE_NAME);

        // @cpt-begin:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3e
        let cfg: ResourceGroupConfig = ctx.config()?;

        self.url_prefix
            .set(cfg.url_prefix)
            .map_err(|_| anyhow::anyhow!("{} url_prefix already set", Self::MODULE_NAME))?;
        // @cpt-end:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3e

        // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-2
        let db = Arc::new(ctx.db_required()?);
        // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-2

        // Resolve AuthZ resolver for PolicyEnforcer (PEP flow)
        let authz = ctx
            .client_hub()
            .get::<dyn AuthZResolverClient>()
            .map_err(|e| anyhow::anyhow!("failed to get AuthZ resolver: {e}"))?;
        // Declare PEP capabilities so PDP can return advanced predicates.
        // TenantHierarchy: `tenant_closure` local projection exists (CDC from tenant-resolver).
        // GroupHierarchy: plugin validates group ownership via ResourceGroupReadHierarchy.
        let enforcer = PolicyEnforcer::new(authz)
            .with_capabilities(vec![Capability::TenantHierarchy, Capability::GroupHierarchy]);

        // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-3
        // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-4
        // @cpt-begin:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3b
        let svc = Arc::new(RgService::new(db, cfg.max_depth, cfg.max_width, enforcer));
        // @cpt-end:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3b
        // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-4
        // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-3

        // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-5
        // @cpt-begin:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3c
        let client: Arc<dyn ResourceGroupClient> = svc.clone();
        ctx.client_hub().register::<dyn ResourceGroupClient>(client);
        // @cpt-end:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3c
        // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-5

        // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-6
        // @cpt-begin:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3d
        let read_hierarchy: Arc<dyn ResourceGroupReadHierarchy> = svc.clone();
        ctx.client_hub()
            .register::<dyn ResourceGroupReadHierarchy>(read_hierarchy);
        // @cpt-end:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-3d
        // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-6

        self.service
            .set(svc)
            .map_err(|_| anyhow::anyhow!("{} module already initialized", Self::MODULE_NAME))?;

        // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-1
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
    // @cpt-begin:cpt-cf-resource-group-algo-phased-init:p1:inst-init-9
    // @cpt-begin:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-5
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
    // @cpt-end:cpt-cf-resource-group-flow-module-bootstrap:p1:inst-bootstrap-5
    // @cpt-end:cpt-cf-resource-group-algo-phased-init:p1:inst-init-9
}
