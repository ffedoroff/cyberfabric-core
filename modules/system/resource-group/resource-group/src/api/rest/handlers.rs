use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::{Extension, Json};
use http::StatusCode;
use modkit::api::problem::Problem;
use modkit_security::SecurityContext;
use resource_group_sdk::{
    AddMembershipRequest, CreateGroupRequest, CreateTypeRequest, ListQuery,
    RemoveMembershipRequest, ResourceGroupClient, UpdateGroupRequest, UpdateTypeRequest,
};
use serde::Deserialize;
use uuid::Uuid;

use super::dto::{
    CreateGroupDto, CreateTypeDto, GroupResponse, GroupWithDepthResponse, MembershipResponse,
    PageInfoResponse, PageResponse, TypeResponse, UpdateGroupDto, UpdateTypeDto,
};
use crate::domain::error::DomainError;
use crate::domain::service::RgService;

type ApiResult<T> = Result<T, Problem>;

/// Query parameters for list endpoints supporting OData-style filtering and pagination.
#[derive(Debug, Default, Deserialize)]
pub struct ListQueryParams {
    #[serde(rename = "$filter")]
    pub filter: Option<String>,
    #[serde(rename = "$top")]
    pub top: Option<i32>,
    #[serde(rename = "$skip")]
    pub skip: Option<i32>,
}

impl From<ListQueryParams> for ListQuery {
    fn from(params: ListQueryParams) -> Self {
        ListQuery {
            filter: params.filter,
            top: params.top,
            skip: params.skip,
        }
    }
}

// ── Type handlers ───────────────────────────────────────────────────────

pub async fn list_types(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Query(params): Query<ListQueryParams>,
) -> ApiResult<Json<PageResponse<TypeResponse>>> {
    let page = svc
        .list_types(&ctx, params.into())
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(to_page_response(page, TypeResponse::from)))
}

pub async fn create_type(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Json(body): Json<CreateTypeDto>,
) -> ApiResult<(StatusCode, Json<TypeResponse>)> {
    let result = svc
        .create_type(
            &ctx,
            CreateTypeRequest {
                code: body.code,
                parents: body.parents,
            },
        )
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok((StatusCode::CREATED, Json(TypeResponse::from(result))))
}

pub async fn get_type(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(code): Path<String>,
) -> ApiResult<Json<TypeResponse>> {
    let result = svc
        .get_type(&ctx, &code)
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(TypeResponse::from(result)))
}

pub async fn update_type(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(code): Path<String>,
    Json(body): Json<UpdateTypeDto>,
) -> ApiResult<Json<TypeResponse>> {
    let result = svc
        .update_type(
            &ctx,
            &code,
            UpdateTypeRequest {
                parents: body.parents,
            },
        )
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(TypeResponse::from(result)))
}

pub async fn delete_type(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(code): Path<String>,
) -> ApiResult<StatusCode> {
    svc.delete_type(&ctx, &code)
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Group handlers ──────────────────────────────────────────────────────

pub async fn list_groups(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
) -> ApiResult<Json<PageResponse<GroupResponse>>> {
    let page = svc
        .list_groups(&ctx, ListQuery::default())
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(to_page_response(page, GroupResponse::from)))
}

pub async fn create_group(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Json(body): Json<CreateGroupDto>,
) -> ApiResult<(StatusCode, Json<GroupResponse>)> {
    let result = svc
        .create_group(
            &ctx,
            CreateGroupRequest {
                group_type: body.group_type,
                name: body.name,
                parent_id: body.parent_id,
                tenant_id: body.tenant_id,
                external_id: body.external_id,
            },
        )
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok((StatusCode::CREATED, Json(GroupResponse::from(result))))
}

pub async fn get_group(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<GroupResponse>> {
    let result = svc
        .get_group(&ctx, group_id)
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(GroupResponse::from(result)))
}

pub async fn update_group(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(group_id): Path<Uuid>,
    Json(body): Json<UpdateGroupDto>,
) -> ApiResult<Json<GroupResponse>> {
    let result = svc
        .update_group(
            &ctx,
            group_id,
            UpdateGroupRequest {
                group_type: body.group_type,
                name: body.name,
                parent_id: body.parent_id,
                external_id: body.external_id,
            },
        )
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(GroupResponse::from(result)))
}

pub async fn delete_group(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(group_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // TODO: parse ?force=true from query params when implementing Feature 3
    svc.delete_group(&ctx, group_id, false)
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_group_depth(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path(group_id): Path<Uuid>,
) -> ApiResult<Json<PageResponse<GroupWithDepthResponse>>> {
    let page = svc
        .list_group_depth(&ctx, group_id, ListQuery::default())
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(to_page_response(page, GroupWithDepthResponse::from)))
}

// ── Membership handlers ─────────────────────────────────────────────────

pub async fn list_memberships(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
) -> ApiResult<Json<PageResponse<MembershipResponse>>> {
    let page = svc
        .list_memberships(&ctx, ListQuery::default())
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(Json(to_page_response(page, MembershipResponse::from)))
}

pub async fn add_membership(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path((group_id, resource_type, resource_id)): Path<(Uuid, String, String)>,
) -> ApiResult<(StatusCode, Json<MembershipResponse>)> {
    let result = svc
        .add_membership(
            &ctx,
            AddMembershipRequest {
                group_id,
                resource_type,
                resource_id,
            },
        )
        .await
        .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok((StatusCode::CREATED, Json(MembershipResponse::from(result))))
}

pub async fn delete_membership(
    Extension(ctx): Extension<SecurityContext>,
    Extension(svc): Extension<Arc<RgService>>,
    Path((group_id, resource_type, resource_id)): Path<(Uuid, String, String)>,
) -> ApiResult<StatusCode> {
    svc.remove_membership(
        &ctx,
        RemoveMembershipRequest {
            group_id,
            resource_type,
            resource_id,
        },
    )
    .await
    .map_err(|e| Problem::from(DomainError::from_sdk_err(e)))?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn to_page_response<T, U: serde::Serialize>(
    page: resource_group_sdk::Page<T>,
    convert: impl Fn(T) -> U,
) -> PageResponse<U> {
    PageResponse {
        items: page.items.into_iter().map(convert).collect(),
        page_info: PageInfoResponse {
            top: page.page_info.top,
            skip: page.page_info.skip,
        },
    }
}

impl DomainError {
    /// Convert SDK error back to domain error for Problem mapping.
    pub(crate) fn from_sdk_err(err: resource_group_sdk::ResourceGroupError) -> Self {
        use resource_group_sdk::ResourceGroupError;
        match err {
            ResourceGroupError::Validation { message } => DomainError::Validation { message },
            ResourceGroupError::NotFound {
                entity_kind,
                identifier,
            } => match entity_kind.as_str() {
                "Type" => DomainError::TypeNotFound { code: identifier },
                "Group" => DomainError::GroupNotFound {
                    id: identifier.parse().unwrap_or_default(),
                },
                _ => DomainError::database(format!("{entity_kind} not found: {identifier}")),
            },
            ResourceGroupError::TypeAlreadyExists { code } => {
                DomainError::TypeAlreadyExists { code }
            }
            ResourceGroupError::InvalidParentType {
                child_type,
                parent_type,
            } => DomainError::InvalidParentType {
                child_type,
                parent_type,
            },
            ResourceGroupError::CycleDetected {
                ancestor_id,
                descendant_id,
            } => DomainError::CycleDetected {
                ancestor_id,
                descendant_id,
            },
            ResourceGroupError::ConflictActiveReferences { reference_count } => {
                DomainError::ActiveReferences {
                    count: reference_count,
                }
            }
            ResourceGroupError::LimitViolation {
                limit_name,
                current_value,
                max_value,
            } => DomainError::LimitViolation {
                limit_name,
                current: current_value,
                max: max_value,
            },
            ResourceGroupError::TenantIncompatibility { message } => {
                DomainError::TenantIncompatibility { message }
            }
            ResourceGroupError::ServiceUnavailable | ResourceGroupError::Internal => {
                DomainError::database("Internal error")
            }
        }
    }
}
