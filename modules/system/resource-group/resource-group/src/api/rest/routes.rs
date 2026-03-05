use std::sync::Arc;

use axum::{Extension, Router};
use modkit::api::OpenApiRegistry;
use modkit::api::operation_builder::{LicenseFeature, OperationBuilder};

use super::dto::{
    CreateGroupDto, CreateTypeDto, GroupResponse, GroupWithDepthResponse, MembershipResponse,
    PageResponse, TypeResponse, UpdateGroupDto, UpdateTypeDto,
};
use super::handlers;
use crate::domain::service::RgService;

struct License;

impl AsRef<str> for License {
    fn as_ref(&self) -> &'static str {
        "gts.x.core.lic.feat.v1~x.core.global.base.v1"
    }
}

impl LicenseFeature for License {}

#[allow(clippy::too_many_lines)]
pub fn register_routes(
    router: Router,
    openapi: &dyn OpenApiRegistry,
    service: Arc<RgService>,
    prefix: &str,
) -> Router {
    let base = format!("{prefix}/v1", prefix = prefix.trim_end_matches('/'));

    // ── Type endpoints ──────────────────────────────────────────────

    let router = OperationBuilder::get(format!("{base}/types"))
        .operation_id("resource_group.list_types")
        .summary("List types")
        .description("List all resource group types with OData query support")
        .tag("resource-group-types")
        .authenticated()
        .require_license_features::<License>([])
        .query_param("$filter", false, "OData filter expression")
        .query_param("$top", false, "Page size (1..300, default 50)")
        .query_param("$skip", false, "Offset (default 0)")
        .handler(handlers::list_types)
        .json_response_with_schema::<PageResponse<TypeResponse>>(
            openapi,
            http::StatusCode::OK,
            "Paginated list of types",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::post(format!("{base}/types"))
        .operation_id("resource_group.create_type")
        .summary("Create type")
        .description("Create a new resource group type")
        .tag("resource-group-types")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<CreateTypeDto>(openapi, "Type creation data")
        .handler(handlers::create_type)
        .json_response_with_schema::<TypeResponse>(
            openapi,
            http::StatusCode::CREATED,
            "Created type",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::get(format!("{base}/types/{{code}}"))
        .operation_id("resource_group.get_type")
        .summary("Get type")
        .description("Get a resource group type by code")
        .tag("resource-group-types")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("code", "Type code")
        .handler(handlers::get_type)
        .json_response_with_schema::<TypeResponse>(openapi, http::StatusCode::OK, "Type details")
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::put(format!("{base}/types/{{code}}"))
        .operation_id("resource_group.update_type")
        .summary("Update type")
        .description("Update a resource group type")
        .tag("resource-group-types")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("code", "Type code")
        .json_request::<UpdateTypeDto>(openapi, "Type update data")
        .handler(handlers::update_type)
        .json_response_with_schema::<TypeResponse>(openapi, http::StatusCode::OK, "Updated type")
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::delete(format!("{base}/types/{{code}}"))
        .operation_id("resource_group.delete_type")
        .summary("Delete type")
        .description("Delete a resource group type")
        .tag("resource-group-types")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("code", "Type code")
        .handler(handlers::delete_type)
        .json_response(http::StatusCode::NO_CONTENT, "Type deleted")
        .standard_errors(openapi)
        .register(router, openapi);

    // ── Group endpoints ─────────────────────────────────────────────

    let router = OperationBuilder::get(format!("{base}/groups"))
        .operation_id("resource_group.list_groups")
        .summary("List groups")
        .description("List resource groups with OData query support")
        .tag("resource-group-groups")
        .authenticated()
        .require_license_features::<License>([])
        .query_param("$filter", false, "OData filter expression")
        .query_param("$top", false, "Page size (1..300, default 50)")
        .query_param("$skip", false, "Offset (default 0)")
        .handler(handlers::list_groups)
        .json_response_with_schema::<PageResponse<GroupResponse>>(
            openapi,
            http::StatusCode::OK,
            "Paginated list of groups",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::post(format!("{base}/groups"))
        .operation_id("resource_group.create_group")
        .summary("Create group")
        .description("Create a new resource group")
        .tag("resource-group-groups")
        .authenticated()
        .require_license_features::<License>([])
        .json_request::<CreateGroupDto>(openapi, "Group creation data")
        .handler(handlers::create_group)
        .json_response_with_schema::<GroupResponse>(
            openapi,
            http::StatusCode::CREATED,
            "Created group",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::get(format!("{base}/groups/{{group_id}}"))
        .operation_id("resource_group.get_group")
        .summary("Get group")
        .description("Get a resource group by ID")
        .tag("resource-group-groups")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("group_id", "Group UUID")
        .handler(handlers::get_group)
        .json_response_with_schema::<GroupResponse>(openapi, http::StatusCode::OK, "Group details")
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::put(format!("{base}/groups/{{group_id}}"))
        .operation_id("resource_group.update_group")
        .summary("Update group")
        .description("Update a resource group")
        .tag("resource-group-groups")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("group_id", "Group UUID")
        .json_request::<UpdateGroupDto>(openapi, "Group update data")
        .handler(handlers::update_group)
        .json_response_with_schema::<GroupResponse>(openapi, http::StatusCode::OK, "Updated group")
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::delete(format!("{base}/groups/{{group_id}}"))
        .operation_id("resource_group.delete_group")
        .summary("Delete group")
        .description("Delete a resource group (optional ?force=true for cascade)")
        .tag("resource-group-groups")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("group_id", "Group UUID")
        .query_param("force", false, "Force cascade deletion")
        .handler(handlers::delete_group)
        .json_response(http::StatusCode::NO_CONTENT, "Group deleted")
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::get(format!("{base}/groups/{{group_id}}/depth"))
        .operation_id("resource_group.list_group_depth")
        .summary("List group depth")
        .description("Traverse hierarchy from reference group with relative depth")
        .tag("resource-group-groups")
        .authenticated()
        .require_license_features::<License>([])
        .path_param("group_id", "Reference group UUID")
        .query_param("$filter", false, "OData filter (depth, group_type)")
        .query_param("$top", false, "Page size (1..300, default 50)")
        .query_param("$skip", false, "Offset (default 0)")
        .handler(handlers::list_group_depth)
        .json_response_with_schema::<PageResponse<GroupWithDepthResponse>>(
            openapi,
            http::StatusCode::OK,
            "Hierarchy traversal with depth",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    // ── Membership endpoints ────────────────────────────────────────

    let router = OperationBuilder::get(format!("{base}/memberships"))
        .operation_id("resource_group.list_memberships")
        .summary("List memberships")
        .description("List resource group memberships with OData query support")
        .tag("resource-group-memberships")
        .authenticated()
        .require_license_features::<License>([])
        .query_param("$filter", false, "OData filter expression")
        .query_param("$top", false, "Page size (1..300, default 50)")
        .query_param("$skip", false, "Offset (default 0)")
        .handler(handlers::list_memberships)
        .json_response_with_schema::<PageResponse<MembershipResponse>>(
            openapi,
            http::StatusCode::OK,
            "Paginated list of memberships",
        )
        .standard_errors(openapi)
        .register(router, openapi);

    let router = OperationBuilder::post(format!(
        "{base}/memberships/{{group_id}}/{{resource_type}}/{{resource_id}}"
    ))
    .operation_id("resource_group.add_membership")
    .summary("Add membership")
    .description("Add a resource to a group")
    .tag("resource-group-memberships")
    .authenticated()
    .require_license_features::<License>([])
    .path_param("group_id", "Group UUID")
    .path_param("resource_type", "Resource type")
    .path_param("resource_id", "Resource identifier")
    .handler(handlers::add_membership)
    .json_response_with_schema::<MembershipResponse>(
        openapi,
        http::StatusCode::CREATED,
        "Created membership",
    )
    .standard_errors(openapi)
    .register(router, openapi);

    let router = OperationBuilder::delete(format!(
        "{base}/memberships/{{group_id}}/{{resource_type}}/{{resource_id}}"
    ))
    .operation_id("resource_group.delete_membership")
    .summary("Delete membership")
    .description("Remove a resource from a group")
    .tag("resource-group-memberships")
    .authenticated()
    .require_license_features::<License>([])
    .path_param("group_id", "Group UUID")
    .path_param("resource_type", "Resource type")
    .path_param("resource_id", "Resource identifier")
    .handler(handlers::delete_membership)
    .json_response(http::StatusCode::NO_CONTENT, "Membership deleted")
    .standard_errors(openapi)
    .register(router, openapi);

    // Attach service as extension for all routes
    router.layer(Extension(service))
}
