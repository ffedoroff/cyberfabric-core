use serde::Serialize;
use uuid::Uuid;

// ── Type DTOs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(response)]
pub struct TypeResponse {
    pub code: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(request)]
pub struct CreateTypeDto {
    pub code: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(request)]
pub struct UpdateTypeDto {
    pub parents: Vec<String>,
}

// ── Group DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(response)]
pub struct GroupResponse {
    pub group_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub group_type: String,
    pub name: String,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(response)]
pub struct GroupWithDepthResponse {
    pub group_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub group_type: String,
    pub name: String,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
    pub depth: i32,
}

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(request)]
pub struct CreateGroupDto {
    pub group_type: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(request)]
pub struct UpdateGroupDto {
    pub group_type: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub external_id: Option<String>,
}

// ── Membership DTOs ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[modkit_macros::api_dto(response)]
pub struct MembershipResponse {
    pub group_id: Uuid,
    pub resource_type: String,
    pub resource_id: String,
}

// ── Pagination DTOs ─────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PageInfoResponse {
    pub top: i32,
    pub skip: i32,
}

impl modkit::api::api_dto::ResponseApiDto for PageInfoResponse {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PageResponse<T: Serialize> {
    pub items: Vec<T>,
    pub page_info: PageInfoResponse,
}

impl<T: Serialize + modkit::api::api_dto::ResponseApiDto> modkit::api::api_dto::ResponseApiDto
    for PageResponse<T>
{
}

// ── Conversions from SDK models ─────────────────────────────────────────

impl From<resource_group_sdk::ResourceGroupType> for TypeResponse {
    fn from(t: resource_group_sdk::ResourceGroupType) -> Self {
        Self {
            code: t.code,
            parents: t.parents,
        }
    }
}

impl From<resource_group_sdk::ResourceGroup> for GroupResponse {
    fn from(g: resource_group_sdk::ResourceGroup) -> Self {
        Self {
            group_id: g.group_id,
            parent_id: g.parent_id,
            group_type: g.group_type,
            name: g.name,
            tenant_id: g.tenant_id,
            external_id: g.external_id,
        }
    }
}

impl From<resource_group_sdk::ResourceGroupWithDepth> for GroupWithDepthResponse {
    fn from(g: resource_group_sdk::ResourceGroupWithDepth) -> Self {
        Self {
            group_id: g.group_id,
            parent_id: g.parent_id,
            group_type: g.group_type,
            name: g.name,
            tenant_id: g.tenant_id,
            external_id: g.external_id,
            depth: g.depth,
        }
    }
}

impl From<resource_group_sdk::ResourceGroupMembership> for MembershipResponse {
    fn from(m: resource_group_sdk::ResourceGroupMembership) -> Self {
        Self {
            group_id: m.group_id,
            resource_type: m.resource_type,
            resource_id: m.resource_id,
        }
    }
}
