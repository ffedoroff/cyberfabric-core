//! Domain service for GTS type management.
//!
//! Implements business rules: input validation, placement invariant,
//! hierarchy safety checks, and CRUD orchestration.

use std::sync::Arc;

use modkit_db::secure::DBRunner;
use resource_group_sdk::models::{CreateTypeRequest, ResourceGroupType, UpdateTypeRequest};

use crate::domain::error::DomainError;
use crate::infra::storage::type_repo::TypeRepository;

/// GTS type path prefix required for resource group types.
const RG_TYPE_PREFIX: &str = "gts.x.system.rg.type.v1~";

/// Type alias for the database provider used by the service.
type DbProvider = modkit_db::DBProvider<modkit_db::DbError>;

// @cpt-dod:cpt-cf-resource-group-dod-type-mgmt-service-crud:p1
/// Service for GTS type lifecycle management.
#[allow(unknown_lints, de0309_must_have_domain_model)]
#[derive(Clone)]
pub struct TypeService {
    db: Arc<DbProvider>,
}

impl TypeService {
    /// Create a new `TypeService` with the given database provider.
    #[must_use]
    pub fn new(db: Arc<DbProvider>) -> Self {
        Self { db }
    }

    // @cpt-flow:cpt-cf-resource-group-flow-type-mgmt-create-type:p1
    /// Create a new GTS type definition.
    pub async fn create_type(
        &self,
        req: CreateTypeRequest,
    ) -> Result<ResourceGroupType, DomainError> {
        Self::validate_type_code(&req.code)?;
        Self::validate_placement_invariant(req.can_be_root, &req.allowed_parents)?;

        let conn = self.db.conn()?;

        // Check uniqueness
        let existing = TypeRepository::find_by_code(&conn, &req.code).await?;
        if existing.is_some() {
            return Err(DomainError::type_already_exists(&req.code));
        }

        // Validate and resolve allowed_parents references
        let parent_ids = if req.allowed_parents.is_empty() {
            Vec::new()
        } else {
            for parent_code in &req.allowed_parents {
                Self::validate_type_code(parent_code)?;
            }
            TypeRepository::resolve_ids(&conn, &req.allowed_parents).await?
        };

        // Validate and resolve allowed_memberships references
        let membership_ids = if req.allowed_memberships.is_empty() {
            Vec::new()
        } else {
            TypeRepository::resolve_ids(&conn, &req.allowed_memberships).await?
        };

        // Persist the type
        let type_model = TypeRepository::insert(
            &conn,
            &req.code,
            req.can_be_root,
            req.metadata_schema.as_ref(),
        )
        .await?;

        // Insert junction entries
        TypeRepository::insert_allowed_parents(&conn, type_model.id, &parent_ids).await?;
        TypeRepository::insert_allowed_memberships(&conn, type_model.id, &membership_ids).await?;

        // Load and return the full type
        TypeRepository::load_full_type(&conn, &type_model).await
    }

    /// Get a GTS type definition by its code (GTS type path).
    pub async fn get_type(&self, code: &str) -> Result<ResourceGroupType, DomainError> {
        let conn = self.db.conn()?;
        TypeRepository::find_by_code(&conn, code)
            .await?
            .ok_or_else(|| DomainError::type_not_found(code))
    }

    /// List GTS type definitions.
    pub async fn list_types(&self) -> Result<Vec<ResourceGroupType>, DomainError> {
        let conn = self.db.conn()?;
        let models = TypeRepository::list_all(&conn).await?;
        let mut types = Vec::with_capacity(models.len());
        for model in &models {
            let rg_type = TypeRepository::load_full_type(&conn, model).await?;
            types.push(rg_type);
        }
        Ok(types)
    }

    // @cpt-flow:cpt-cf-resource-group-flow-type-mgmt-update-type:p1
    /// Update a GTS type definition (full replacement).
    pub async fn update_type(
        &self,
        code: &str,
        req: UpdateTypeRequest,
    ) -> Result<ResourceGroupType, DomainError> {
        let conn = self.db.conn()?;

        // Load existing type
        let existing = TypeRepository::find_by_code(&conn, code)
            .await?
            .ok_or_else(|| DomainError::type_not_found(code))?;

        // Validate placement invariant on new values
        Self::validate_placement_invariant(req.can_be_root, &req.allowed_parents)?;

        // Validate and resolve references
        let parent_ids = if req.allowed_parents.is_empty() {
            Vec::new()
        } else {
            for parent_code in &req.allowed_parents {
                Self::validate_type_code(parent_code)?;
            }
            TypeRepository::resolve_ids(&conn, &req.allowed_parents).await?
        };

        let membership_ids = if req.allowed_memberships.is_empty() {
            Vec::new()
        } else {
            TypeRepository::resolve_ids(&conn, &req.allowed_memberships).await?
        };

        // Resolve our own ID
        let type_id = TypeRepository::resolve_id(&conn, code)
            .await?
            .ok_or_else(|| DomainError::type_not_found(code))?;

        // Hierarchy safety check
        Self::check_hierarchy_safety(&conn, type_id, &existing, &req).await?;

        // Clear old junction entries, insert new ones, update type
        TypeRepository::delete_allowed_parents(&conn, type_id).await?;
        TypeRepository::insert_allowed_parents(&conn, type_id, &parent_ids).await?;

        TypeRepository::delete_allowed_memberships(&conn, type_id).await?;
        TypeRepository::insert_allowed_memberships(&conn, type_id, &membership_ids).await?;

        let updated_model = TypeRepository::update_type(
            &conn,
            type_id,
            code,
            req.can_be_root,
            req.metadata_schema.as_ref(),
        )
        .await?;

        TypeRepository::load_full_type(&conn, &updated_model).await
    }

    // @cpt-flow:cpt-cf-resource-group-flow-type-mgmt-delete-type:p1
    /// Delete a GTS type definition.
    pub async fn delete_type(&self, code: &str) -> Result<(), DomainError> {
        let conn = self.db.conn()?;

        let type_id = TypeRepository::resolve_id(&conn, code)
            .await?
            .ok_or_else(|| DomainError::type_not_found(code))?;

        // Check for active references
        let count = TypeRepository::count_groups_of_type(&conn, type_id).await?;
        if count > 0 {
            return Err(DomainError::conflict_active_references(format!(
                "Cannot delete type '{code}': {count} group(s) of this type exist"
            )));
        }

        TypeRepository::delete_by_id(&conn, type_id).await
    }

    // -- Validation helpers --

    // @cpt-algo:cpt-cf-resource-group-algo-sdk-foundation-validate-gts-type-path:p1
    // @cpt-algo:cpt-cf-resource-group-algo-type-mgmt-validate-type-input:p1
    fn validate_type_code(code: &str) -> Result<(), DomainError> {
        if code.is_empty() {
            return Err(DomainError::validation("Type code must not be empty"));
        }
        if !code.starts_with(RG_TYPE_PREFIX) {
            return Err(DomainError::validation(format!(
                "Type code must start with prefix '{RG_TYPE_PREFIX}', got: '{code}'"
            )));
        }
        if code.len() > 1024 {
            return Err(DomainError::validation(
                "Type code must not exceed 1024 characters",
            ));
        }
        Ok(())
    }

    fn validate_placement_invariant(
        can_be_root: bool,
        allowed_parents: &[String],
    ) -> Result<(), DomainError> {
        if !can_be_root && allowed_parents.is_empty() {
            return Err(DomainError::validation(
                "Type must allow root placement or have at least one allowed parent",
            ));
        }
        Ok(())
    }

    // @cpt-algo:cpt-cf-resource-group-algo-type-mgmt-check-hierarchy-safety:p1
    async fn check_hierarchy_safety(
        conn: &impl DBRunner,
        type_id: i16,
        existing: &ResourceGroupType,
        req: &UpdateTypeRequest,
    ) -> Result<(), DomainError> {
        // Check removed parents
        let removed_parents: Vec<&String> = existing
            .allowed_parents
            .iter()
            .filter(|p| !req.allowed_parents.contains(p))
            .collect();

        for removed_parent in &removed_parents {
            let parent_id = TypeRepository::resolve_id(conn, removed_parent).await?;
            if let Some(parent_id) = parent_id {
                let violations =
                    TypeRepository::find_groups_using_parent_type(conn, type_id, parent_id).await?;

                if !violations.is_empty() {
                    let names: Vec<String> =
                        violations.iter().map(|(_, name)| name.clone()).collect();
                    return Err(DomainError::allowed_parents_violation(format!(
                        "Cannot remove allowed parent '{removed_parent}': groups using this parent relationship: {}",
                        names.join(", ")
                    )));
                }
            }
        }

        // Check can_be_root change from true to false
        if existing.can_be_root && !req.can_be_root {
            let root_groups = TypeRepository::find_root_groups_of_type(conn, type_id).await?;

            if !root_groups.is_empty() {
                let names: Vec<String> = root_groups.iter().map(|(_, name)| name.clone()).collect();
                return Err(DomainError::allowed_parents_violation(format!(
                    "Cannot disable root placement: root groups of this type exist: {}",
                    names.join(", ")
                )));
            }
        }

        Ok(())
    }
}
