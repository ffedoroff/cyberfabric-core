//! REST API route definitions using `OperationBuilder`.

use crate::api::rest::{dto, handlers};
use crate::domain::group_service::GroupService;
use crate::domain::type_service::TypeService;
use axum::Router;
use modkit::api::OpenApiRegistry;
use std::sync::Arc;

mod groups;
mod types;

/// Register all routes for the resource-group module.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn register_routes(
    mut router: Router,
    openapi: &dyn OpenApiRegistry,
    type_service: Arc<TypeService>,
    group_service: Arc<GroupService>,
) -> Router {
    router = types::register_type_routes(router, openapi);
    router = groups::register_group_routes(router, openapi);

    router = router
        .layer(axum::Extension(type_service))
        .layer(axum::Extension(group_service));

    router
}
