//! Domain service for resource group entity management.
//!
//! Implements business rules: type validation, parent compatibility,
//! cycle detection, closure table management, query profile enforcement,
//! and CRUD orchestration.

use std::sync::Arc;

use authz_resolver_sdk::pep::{PolicyEnforcer, ResourceType};
use modkit_db::secure::DBRunner;
use modkit_odata::{ODataQuery, Page};
use modkit_security::{SecurityContext, pep_properties};
use resource_group_sdk::models::{
    CreateGroupRequest, ResourceGroup, ResourceGroupWithDepth, UpdateGroupRequest,
};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::infra::storage::group_repo::GroupRepository;
use crate::infra::storage::type_repo::TypeRepository;

/// `AuthZ` resource type descriptor for resource groups.
pub const RG_GROUP_RESOURCE: ResourceType = ResourceType {
    name: "gts.x.system.rg.group.v1~",
    supported_properties: &[pep_properties::OWNER_TENANT_ID, pep_properties::RESOURCE_ID],
};

/// GTS type path prefix required for resource group types.
const RG_TYPE_PREFIX: &str = "gts.x.system.rg.type.v1~";

/// Type alias for the database provider used by the service.
type DbProvider = modkit_db::DBProvider<modkit_db::DbError>;

/// Query profile configuration for depth/width limits.
#[allow(unknown_lints, de0309_must_have_domain_model)]
#[derive(Debug, Clone)]
pub struct QueryProfile {
    /// Maximum depth allowed. `None` disables depth limit.
    pub max_depth: Option<u32>,
    /// Maximum width (children per parent) allowed. `None` disables width limit.
    pub max_width: Option<u32>,
}

impl Default for QueryProfile {
    fn default() -> Self {
        Self {
            max_depth: Some(10),
            max_width: None,
        }
    }
}

// @cpt-dod:cpt-cf-resource-group-dod-entity-hier-entity-service:p1
// @cpt-dod:cpt-cf-resource-group-dod-integration-auth-tenant-scope:p1
/// Service for resource group entity lifecycle management.
#[allow(unknown_lints, de0309_must_have_domain_model)]
#[derive(Clone)]
pub struct GroupService {
    db: Arc<DbProvider>,
    profile: QueryProfile,
    enforcer: PolicyEnforcer,
}

impl GroupService {
    /// Create a new `GroupService` with the given database provider, query profile,
    /// and `PolicyEnforcer` for AuthZ-scoped queries.
    #[must_use]
    pub fn new(db: Arc<DbProvider>, profile: QueryProfile, enforcer: PolicyEnforcer) -> Self {
        Self {
            db,
            profile,
            enforcer,
        }
    }

    // @cpt-flow:cpt-cf-resource-group-flow-entity-hier-create-group:p1
    /// Create a new resource group.
    pub async fn create_group(
        &self,
        req: CreateGroupRequest,
        tenant_id: Uuid,
    ) -> Result<ResourceGroup, DomainError> {
        Self::validate_type_code(&req.type_path)?;
        Self::validate_name(&req.name)?;

        let conn = self.db.conn()?;

        // Resolve type
        let type_id = TypeRepository::resolve_id(&conn, &req.type_path)
            .await?
            .ok_or_else(|| DomainError::type_not_found(&req.type_path))?;

        // Load full type for validation
        let rg_type = TypeRepository::find_by_code(&conn, &req.type_path)
            .await?
            .ok_or_else(|| DomainError::type_not_found(&req.type_path))?;

        if let Some(parent_id) = req.parent_id {
            // Child group: validate parent exists, type compatibility, tenant
            let parent = GroupRepository::find_model_by_id(&conn, parent_id)
                .await?
                .ok_or_else(|| DomainError::group_not_found(parent_id))?;

            // Validate parent type compatibility
            let parent_type_path =
                Self::resolve_type_path_from_id(&conn, parent.gts_type_id).await?;
            if !rg_type.allowed_parents.contains(&parent_type_path) {
                return Err(DomainError::invalid_parent_type(format!(
                    "Type '{}' does not allow parent type '{}'",
                    req.type_path, parent_type_path
                )));
            }

            // @cpt-algo:cpt-cf-resource-group-algo-integration-auth-tenant-scope-enforcement:p1
            // Validate tenant compatibility (child must be same tenant as parent)
            if parent.tenant_id != tenant_id {
                return Err(DomainError::validation(format!(
                    "Child group tenant_id ({tenant_id}) must match parent tenant_id ({})",
                    parent.tenant_id
                )));
            }

            // Check query profile: depth limit
            if let Some(max_depth) = self.profile.max_depth {
                let parent_depth = GroupRepository::get_depth(&conn, parent_id).await?;
                #[allow(clippy::cast_possible_wrap)]
                if parent_depth + 1 >= max_depth as i32 {
                    return Err(DomainError::limit_violation(format!(
                        "Depth limit exceeded: adding child at depth {} exceeds max_depth {}",
                        parent_depth + 1,
                        max_depth
                    )));
                }
            }

            // Check query profile: width limit
            if let Some(max_width) = self.profile.max_width {
                let sibling_count = GroupRepository::count_children(&conn, parent_id).await?;
                if sibling_count >= u64::from(max_width) {
                    return Err(DomainError::limit_violation(format!(
                        "Width limit exceeded: parent already has {sibling_count} children, max_width is {max_width}"
                    )));
                }
            }

            // Insert group
            let group_id = Uuid::now_v7();
            let _model = GroupRepository::insert(
                &conn,
                group_id,
                Some(parent_id),
                type_id,
                &req.name,
                req.metadata.as_ref(),
                tenant_id,
            )
            .await?;

            // Insert closure: self-row + ancestor rows
            GroupRepository::insert_closure_self_row(&conn, group_id).await?;
            GroupRepository::insert_ancestor_closure_rows(&conn, group_id, parent_id).await?;

            let sys = modkit_security::AccessScope::allow_all();
            GroupRepository::find_by_id(&conn, &sys, group_id)
                .await?
                .ok_or_else(|| DomainError::database("Insert succeeded but group not found"))
        } else {
            // Root group: validate can_be_root
            if !rg_type.can_be_root {
                return Err(DomainError::invalid_parent_type(format!(
                    "Type '{}' cannot be a root group (can_be_root=false)",
                    req.type_path
                )));
            }

            // Insert group
            let group_id = Uuid::now_v7();
            let _model = GroupRepository::insert(
                &conn,
                group_id,
                None,
                type_id,
                &req.name,
                req.metadata.as_ref(),
                tenant_id,
            )
            .await?;

            // Insert closure: self-row only
            GroupRepository::insert_closure_self_row(&conn, group_id).await?;

            let sys = modkit_security::AccessScope::allow_all();
            GroupRepository::find_by_id(&conn, &sys, group_id)
                .await?
                .ok_or_else(|| DomainError::database("Insert succeeded but group not found"))
        }
    }

    /// Get a resource group by ID (AuthZ-scoped).
    pub async fn get_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
    ) -> Result<ResourceGroup, DomainError> {
        let scope = self
            .enforcer
            .access_scope(ctx, &RG_GROUP_RESOURCE, "get", Some(group_id))
            .await
            .map_err(DomainError::from)?;
        let conn = self.db.conn()?;
        GroupRepository::find_by_id(&conn, &scope, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))
    }

    /// List resource groups with `OData` filtering and pagination (AuthZ-scoped).
    pub async fn list_groups(
        &self,
        ctx: &SecurityContext,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroup>, DomainError> {
        let scope = self
            .enforcer
            .access_scope(ctx, &RG_GROUP_RESOURCE, "list", None)
            .await
            .map_err(DomainError::from)?;
        let conn = self.db.conn()?;
        GroupRepository::list_groups(&conn, &scope, query).await
    }

    // @cpt-flow:cpt-cf-resource-group-flow-entity-hier-update-group:p1
    /// Update a resource group (full replacement via PUT, AuthZ-scoped).
    pub async fn update_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        req: UpdateGroupRequest,
    ) -> Result<ResourceGroup, DomainError> {
        // AuthZ gate: verify the caller can update this group (tenant check)
        let scope = self
            .enforcer
            .access_scope(ctx, &RG_GROUP_RESOURCE, "update", Some(group_id))
            .await
            .map_err(DomainError::from)?;

        Self::validate_type_code(&req.type_path)?;
        Self::validate_name(&req.name)?;

        let conn = self.db.conn()?;

        // Verify the group is visible under the caller's scope
        GroupRepository::find_by_id(&conn, &scope, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))?;

        // Load raw model for internal validation (system scope)
        let existing = GroupRepository::find_model_by_id(&conn, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))?;

        // Resolve new type
        let new_type_id = TypeRepository::resolve_id(&conn, &req.type_path)
            .await?
            .ok_or_else(|| DomainError::type_not_found(&req.type_path))?;

        let rg_type = TypeRepository::find_by_code(&conn, &req.type_path)
            .await?
            .ok_or_else(|| DomainError::type_not_found(&req.type_path))?;

        // Determine if parent changed
        let parent_changed = existing.parent_id != req.parent_id;
        let type_changed = existing.gts_type_id != new_type_id;

        if type_changed {
            // Validate new type against current parent
            if let Some(parent_id) = existing.parent_id.or(req.parent_id) {
                let parent = GroupRepository::find_model_by_id(&conn, parent_id)
                    .await?
                    .ok_or_else(|| DomainError::group_not_found(parent_id))?;

                let parent_type_path =
                    Self::resolve_type_path_from_id(&conn, parent.gts_type_id).await?;
                if !rg_type.allowed_parents.contains(&parent_type_path) {
                    return Err(DomainError::invalid_parent_type(format!(
                        "New type '{}' does not allow current parent type '{}'",
                        req.type_path, parent_type_path
                    )));
                }
            } else if !rg_type.can_be_root {
                return Err(DomainError::invalid_parent_type(format!(
                    "New type '{}' cannot be a root group",
                    req.type_path
                )));
            }

            // Validate that all children's types allow the new type as parent
            let children = Self::get_direct_children(&conn, group_id).await?;
            for child in &children {
                let child_type = TypeRepository::find_by_code(
                    &conn,
                    &Self::resolve_type_path_from_id(&conn, child.gts_type_id).await?,
                )
                .await?
                .ok_or_else(|| DomainError::database("Child type not found during validation"))?;

                if !child_type.allowed_parents.contains(&req.type_path) {
                    return Err(DomainError::invalid_parent_type(format!(
                        "Child group '{}' of type '{}' does not allow '{}' as parent type",
                        child.name, child_type.code, req.type_path
                    )));
                }
            }
        }

        if parent_changed {
            // Delegate to move_group logic
            self.move_group_internal(&conn, group_id, req.parent_id, &rg_type)
                .await?;
        }

        // Update the group record
        let _model = GroupRepository::update(
            &conn,
            group_id,
            req.parent_id,
            new_type_id,
            &req.name,
            req.metadata.as_ref(),
        )
        .await?;

        let sys = modkit_security::AccessScope::allow_all();
        GroupRepository::find_by_id(&conn, &sys, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))
    }

    // @cpt-flow:cpt-cf-resource-group-flow-entity-hier-move-group:p1
    /// Move a group to a new parent (or make it a root).
    pub async fn move_group(
        &self,
        group_id: Uuid,
        new_parent_id: Option<Uuid>,
    ) -> Result<ResourceGroup, DomainError> {
        let conn = self.db.conn()?;

        let existing = GroupRepository::find_model_by_id(&conn, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))?;

        let type_path = Self::resolve_type_path_from_id(&conn, existing.gts_type_id).await?;
        let rg_type = TypeRepository::find_by_code(&conn, &type_path)
            .await?
            .ok_or_else(|| DomainError::type_not_found(&type_path))?;

        self.move_group_internal(&conn, group_id, new_parent_id, &rg_type)
            .await?;

        // Update parent_id on the group
        GroupRepository::update(
            &conn,
            group_id,
            new_parent_id,
            existing.gts_type_id,
            &existing.name,
            existing.metadata.as_ref(),
        )
        .await?;

        let sys = modkit_security::AccessScope::allow_all();
        GroupRepository::find_by_id(&conn, &sys, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))
    }

    // @cpt-flow:cpt-cf-resource-group-flow-entity-hier-delete-group:p1
    /// Delete a resource group (AuthZ-scoped).
    pub async fn delete_group(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        force: bool,
    ) -> Result<(), DomainError> {
        // AuthZ gate: verify the caller can delete this group (tenant check)
        let scope = self
            .enforcer
            .access_scope(ctx, &RG_GROUP_RESOURCE, "delete", Some(group_id))
            .await
            .map_err(DomainError::from)?;

        let conn = self.db.conn()?;

        // Verify the group is visible under the caller's scope
        GroupRepository::find_by_id(&conn, &scope, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))?;

        let _existing = GroupRepository::find_model_by_id(&conn, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))?;

        if force {
            // Force delete: cascade entire subtree + memberships + closure
            self.force_delete_subtree(&conn, group_id).await
        } else {
            // Non-force: check children and memberships
            let children = Self::get_direct_children(&conn, group_id).await?;
            if !children.is_empty() {
                return Err(DomainError::conflict_active_references(format!(
                    "Cannot delete group '{group_id}': has {} child group(s). Use force=true to cascade.",
                    children.len()
                )));
            }

            let has_memberships = GroupRepository::has_memberships(&conn, group_id).await?;
            if has_memberships {
                return Err(DomainError::conflict_active_references(format!(
                    "Cannot delete group '{group_id}': has active memberships. Use force=true to cascade."
                )));
            }

            // Delete closure rows, then the group
            GroupRepository::delete_all_closure_rows(&conn, group_id).await?;
            GroupRepository::delete_by_id(&conn, group_id).await
        }
    }

    /// List hierarchy for a group (AuthZ-scoped).
    pub async fn list_group_hierarchy(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, DomainError> {
        let scope = self
            .enforcer
            .access_scope(ctx, &RG_GROUP_RESOURCE, "list", Some(group_id))
            .await
            .map_err(DomainError::from)?;
        let conn = self.db.conn()?;

        // Verify group exists
        let _existing = GroupRepository::find_model_by_id(&conn, group_id)
            .await?
            .ok_or_else(|| DomainError::group_not_found(group_id))?;

        GroupRepository::list_hierarchy(&conn, &scope, group_id, query).await
    }

    // -- Internal helpers --

    // @cpt-algo:cpt-cf-resource-group-algo-entity-hier-cycle-detect:p1
    // @cpt-algo:cpt-cf-resource-group-algo-entity-hier-enforce-query-profile:p1
    /// Internal move logic shared between `move_group` and `update_group`.
    async fn move_group_internal(
        &self,
        conn: &impl DBRunner,
        group_id: Uuid,
        new_parent_id: Option<Uuid>,
        rg_type: &resource_group_sdk::ResourceGroupType,
    ) -> Result<(), DomainError> {
        if let Some(new_pid) = new_parent_id {
            // Cycle detection: check new parent is not in subtree of group being moved
            let is_desc = GroupRepository::is_descendant(conn, group_id, new_pid).await?;
            if is_desc {
                return Err(DomainError::cycle_detected(format!(
                    "Cannot move group '{group_id}' under '{new_pid}': would create a cycle"
                )));
            }

            // Validate parent type compatibility
            let parent = GroupRepository::find_model_by_id(conn, new_pid)
                .await?
                .ok_or_else(|| DomainError::group_not_found(new_pid))?;

            let parent_type_path =
                Self::resolve_type_path_from_id(conn, parent.gts_type_id).await?;
            if !rg_type.allowed_parents.contains(&parent_type_path) {
                return Err(DomainError::invalid_parent_type(format!(
                    "Type '{}' does not allow parent type '{}'",
                    rg_type.code, parent_type_path
                )));
            }

            // Check query profile: depth limit
            if let Some(max_depth) = self.profile.max_depth {
                let parent_depth = GroupRepository::get_depth(conn, new_pid).await?;
                // Check depth of deepest descendant of moved node
                let subtree_descendants =
                    GroupRepository::get_descendant_ids(conn, group_id).await?;
                let mut max_subtree_depth = 0i32;
                for desc_id in &subtree_descendants {
                    // Internal depth within the subtree
                    let is_desc_result =
                        GroupRepository::is_descendant(conn, group_id, *desc_id).await?;
                    if is_desc_result {
                        // Get the depth of this descendant relative to the moved group
                        // by looking at the closure table
                        let depth = Self::get_relative_depth(conn, group_id, *desc_id).await?;
                        if depth > max_subtree_depth {
                            max_subtree_depth = depth;
                        }
                    }
                }
                let new_deepest = parent_depth + 1 + max_subtree_depth;
                #[allow(clippy::cast_possible_wrap)]
                if new_deepest >= max_depth as i32 {
                    return Err(DomainError::limit_violation(format!(
                        "Depth limit exceeded: moving subtree would create depth {new_deepest}, max_depth is {max_depth}"
                    )));
                }
            }

            // Check query profile: width limit
            if let Some(max_width) = self.profile.max_width {
                let sibling_count = GroupRepository::count_children(conn, new_pid).await?;
                if sibling_count >= u64::from(max_width) {
                    return Err(DomainError::limit_violation(format!(
                        "Width limit exceeded: new parent already has {sibling_count} children, max_width is {max_width}"
                    )));
                }
            }
        } else if !rg_type.can_be_root {
            // Moving to root: validate can_be_root
            return Err(DomainError::invalid_parent_type(format!(
                "Type '{}' cannot be a root group (can_be_root=false)",
                rg_type.code
            )));
        }

        // Rebuild closure table for the subtree
        GroupRepository::rebuild_subtree_closure(conn, group_id, new_parent_id).await?;

        Ok(())
    }

    /// Force-delete an entire subtree (group + descendants + memberships + closure).
    async fn force_delete_subtree(
        &self,
        conn: &impl DBRunner,
        root_id: Uuid,
    ) -> Result<(), DomainError> {
        // Get all descendants
        let descendant_ids = GroupRepository::get_descendant_ids(conn, root_id).await?;

        // Delete in reverse order (leaves first)
        let mut all_ids = vec![root_id];
        all_ids.extend(descendant_ids);

        // Delete memberships and closure rows for all nodes
        for &gid in all_ids.iter().rev() {
            GroupRepository::delete_memberships(conn, gid).await?;
            GroupRepository::delete_all_closure_rows(conn, gid).await?;
        }

        // Delete group entities in reverse order (leaves first)
        for &gid in all_ids.iter().rev() {
            GroupRepository::delete_by_id(conn, gid).await?;
        }

        Ok(())
    }

    /// Get direct children of a group.
    async fn get_direct_children(
        conn: &impl DBRunner,
        parent_id: Uuid,
    ) -> Result<Vec<crate::infra::storage::entity::resource_group::Model>, DomainError> {
        use modkit_db::secure::SecureEntityExt;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let scope = modkit_security::AccessScope::allow_all();
        crate::infra::storage::entity::resource_group::Entity::find()
            .filter(crate::infra::storage::entity::resource_group::Column::ParentId.eq(parent_id))
            .secure()
            .scope_with(&scope)
            .all(conn)
            .await
            .map_err(|e| DomainError::database(e.to_string()))
    }

    /// Get relative depth between an ancestor and descendant via closure table.
    async fn get_relative_depth(
        conn: &impl DBRunner,
        ancestor_id: Uuid,
        descendant_id: Uuid,
    ) -> Result<i32, DomainError> {
        use modkit_db::secure::SecureEntityExt;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let scope = modkit_security::AccessScope::allow_all();
        let row = crate::infra::storage::entity::resource_group_closure::Entity::find()
            .filter(
                crate::infra::storage::entity::resource_group_closure::Column::AncestorId
                    .eq(ancestor_id),
            )
            .filter(
                crate::infra::storage::entity::resource_group_closure::Column::DescendantId
                    .eq(descendant_id),
            )
            .secure()
            .scope_with(&scope)
            .one(conn)
            .await
            .map_err(|e| DomainError::database(e.to_string()))?;

        Ok(row.map_or(0, |r| r.depth))
    }

    /// Resolve a type ID to its GTS path.
    async fn resolve_type_path_from_id(
        conn: &impl DBRunner,
        type_id: i16,
    ) -> Result<String, DomainError> {
        use modkit_db::secure::SecureEntityExt;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let scope = modkit_security::AccessScope::allow_all();
        let model = crate::infra::storage::entity::gts_type::Entity::find()
            .filter(crate::infra::storage::entity::gts_type::Column::Id.eq(type_id))
            .secure()
            .scope_with(&scope)
            .one(conn)
            .await
            .map_err(|e| DomainError::database(e.to_string()))?
            .ok_or_else(|| DomainError::database(format!("Type ID {type_id} not found")))?;
        Ok(model.schema_id)
    }

    fn validate_type_code(code: &str) -> Result<(), DomainError> {
        if code.is_empty() {
            return Err(DomainError::validation("Type code must not be empty"));
        }
        if !code.starts_with(RG_TYPE_PREFIX) {
            return Err(DomainError::validation(format!(
                "Type code must start with prefix '{RG_TYPE_PREFIX}', got: '{code}'"
            )));
        }
        Ok(())
    }

    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.is_empty() || name.len() > 255 {
            return Err(DomainError::validation(
                "Group name must be between 1 and 255 characters",
            ));
        }
        Ok(())
    }
}
