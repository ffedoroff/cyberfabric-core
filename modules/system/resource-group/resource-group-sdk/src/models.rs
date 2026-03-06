// @cpt-req:cpt-cf-resource-group-dod-sdk-crate:p1

use uuid::Uuid;

// ── Type ────────────────────────────────────────────────────────────────

/// Resource group type definition with allowed parent types.
/// Matches REST `Type` schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGroupType {
    pub code: String,
    pub parents: Vec<String>,
}

/// Request to create a new resource group type.
/// Matches REST `CreateTypeRequest` schema.
#[derive(Debug, Clone)]
pub struct CreateTypeRequest {
    pub code: String,
    pub parents: Vec<String>,
}

/// Request to update an existing resource group type.
/// Matches REST `UpdateTypeRequest` schema.
#[derive(Debug, Clone)]
pub struct UpdateTypeRequest {
    pub parents: Vec<String>,
}

// ── Group ───────────────────────────────────────────────────────────────

/// Resource group entity with optional parent and tenant scope.
/// Matches REST `Group` schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGroup {
    pub group_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub group_type: String,
    pub name: String,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
}

/// Resource group with hierarchy depth relative to a reference group.
/// Matches REST `GroupWithDepth` schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGroupWithDepth {
    pub group_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub group_type: String,
    pub name: String,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
    pub depth: i32,
}

/// Request to create a new resource group.
/// Matches REST `CreateGroupRequest` schema.
#[derive(Debug, Clone)]
pub struct CreateGroupRequest {
    pub group_type: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub tenant_id: Uuid,
    pub external_id: Option<String>,
}

/// Request to update an existing resource group.
/// Matches REST `UpdateGroupRequest` schema.
#[derive(Debug, Clone)]
pub struct UpdateGroupRequest {
    pub group_type: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub external_id: Option<String>,
}

// ── Membership ──────────────────────────────────────────────────────────

/// Resource-to-group membership link.
/// Matches REST `Membership` schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGroupMembership {
    pub group_id: Uuid,
    pub resource_type: String,
    pub resource_id: String,
}

/// Request to add a membership link.
/// Matches REST `addMembership` path params.
#[derive(Debug, Clone)]
pub struct AddMembershipRequest {
    pub group_id: Uuid,
    pub resource_type: String,
    pub resource_id: String,
}

/// Request to remove a membership link.
/// Matches REST `deleteMembership` path params.
#[derive(Debug, Clone)]
pub struct RemoveMembershipRequest {
    pub group_id: Uuid,
    pub resource_type: String,
    pub resource_id: String,
}

// ── Pagination ──────────────────────────────────────────────────────────

/// Pagination metadata for paginated responses.
/// Matches REST `PageInfo` schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub top: i32,
    pub skip: i32,
}

/// Generic paginated response wrapper.
/// Matches REST `*Page` schemas.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page_info: PageInfo,
}

// ── Query ───────────────────────────────────────────────────────────────

/// Lightweight query parameters for SDK trait methods.
/// Converted to `ODataQuery` at the module boundary.
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    pub filter: Option<String>,
    pub top: Option<i32>,
    pub skip: Option<i32>,
}

impl ListQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn filter(mut self, expr: &str) -> Self {
        self.filter = Some(expr.to_owned());
        self
    }

    #[must_use]
    pub fn top(mut self, top: i32) -> Self {
        self.top = Some(top);
        self
    }

    #[must_use]
    pub fn skip(mut self, skip: i32) -> Self {
        self.skip = Some(skip);
        self
    }
}
