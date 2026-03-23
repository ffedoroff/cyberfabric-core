//! Domain service for resource group membership management.
//!
//! Implements business rules for adding, removing, and listing memberships
//! between resources and groups. Delegates persistence to the infra layer.

use std::sync::Arc;

use modkit_odata::{ODataQuery, Page};
use resource_group_sdk::models::ResourceGroupMembership;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::infra::storage::group_repo::GroupRepository;
use crate::infra::storage::membership_repo::MembershipRepository;
use crate::infra::storage::type_repo::TypeRepository;

/// Type alias for the database provider used by the service.
type DbProvider = modkit_db::DBProvider<modkit_db::DbError>;

// @cpt-flow:cpt-cf-resource-group-flow-membership-add:p1
// @cpt-flow:cpt-cf-resource-group-flow-membership-remove:p1
// @cpt-flow:cpt-cf-resource-group-flow-membership-list:p1
// @cpt-dod:cpt-cf-resource-group-dod-membership-service:p1

/// Service for resource group membership lifecycle management.
#[allow(unknown_lints, de0309_must_have_domain_model)]
#[derive(Clone)]
pub struct MembershipService {
    db: Arc<DbProvider>,
}

impl MembershipService {
    /// Create a new `MembershipService` with the given database provider.
    #[must_use]
    pub fn new(db: Arc<DbProvider>) -> Self {
        Self { db }
    }

    fn conn(&self) -> Result<impl modkit_db::secure::DBRunner + '_, DomainError> {
        self.db
            .conn()
            .map_err(|e| DomainError::database(e.to_string()))
    }

    // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-mbr-add-1
    /// Add a membership link between a resource and a group.
    ///
    /// Validates group existence, `resource_type` registration, `allowed_memberships`
    /// compatibility, and tenant scope before inserting the membership row.
    pub async fn add_membership(
        &self,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<ResourceGroupMembership, DomainError> {
        let conn = self.conn()?;

        // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-3
        // Verify the group exists and get its type info
        let group_model = GroupRepository::find_model_by_id(&conn, group_id)
            .await?
            .ok_or(DomainError::GroupNotFound { id: group_id })?;
        // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-3

        // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-5
        // Resolve the GTS type path to a surrogate SMALLINT ID
        let gts_type_id = TypeRepository::resolve_id(&conn, resource_type)
            .await?
            .ok_or_else(|| {
                DomainError::validation(format!("Unknown resource type: {resource_type}"))
            })?;
        // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-5

        // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-7
        // Load group type's allowed_memberships and validate
        let allowed = TypeRepository::load_full_type_by_id(&conn, group_model.gts_type_id).await?;
        // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-7

        // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-8
        if !allowed
            .allowed_memberships
            .iter()
            .any(|m| m == resource_type)
        {
            return Err(DomainError::validation(format!(
                "Resource type '{resource_type}' is not in allowed_memberships for group type '{}'",
                allowed.code
            )));
        }
        // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-8

        // @cpt-begin:cpt-cf-resource-group-algo-membership-check-tenant-compat:p1:inst-tenant-check-1
        // Tenant compatibility: check existing memberships for this resource
        let existing_tenants = MembershipRepository::get_existing_membership_tenant_ids(
            &conn,
            gts_type_id,
            resource_id,
        )
        .await?;
        // @cpt-end:cpt-cf-resource-group-algo-membership-check-tenant-compat:p1:inst-tenant-check-1

        // @cpt-begin:cpt-cf-resource-group-algo-membership-check-tenant-compat:p1:inst-tenant-check-4
        if !existing_tenants.is_empty() && !existing_tenants.contains(&group_model.tenant_id) {
            return Err(DomainError::tenant_incompatibility(format!(
                "Resource ({resource_type}, {resource_id}) is already linked in tenant {:?}, cannot add to tenant {}",
                existing_tenants, group_model.tenant_id
            )));
        }
        // @cpt-end:cpt-cf-resource-group-algo-membership-check-tenant-compat:p1:inst-tenant-check-4

        // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-11
        // Insert the membership (repo handles duplicate detection)
        let model = MembershipRepository::insert(&conn, group_id, gts_type_id, resource_id).await?;
        // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-add-memb-11

        // Resolve back to GTS path for the SDK model
        Ok(ResourceGroupMembership {
            group_id: model.group_id,
            resource_type: resource_type.to_owned(),
            resource_id: model.resource_id,
        })
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-mbr-add-1

    // @cpt-begin:cpt-cf-resource-group-flow-membership-remove:p1:inst-mbr-remove-1
    /// Remove a membership link.
    ///
    /// Resolves the GTS type path, verifies the membership exists, and deletes it.
    pub async fn remove_membership(
        &self,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), DomainError> {
        let conn = self.conn()?;

        // Resolve the GTS type path to a surrogate SMALLINT ID
        let gts_type_id = TypeRepository::resolve_id(&conn, resource_type)
            .await?
            .ok_or_else(|| {
                DomainError::validation(format!("Unknown resource type: {resource_type}"))
            })?;

        // Verify the membership exists
        MembershipRepository::find_by_composite_key(&conn, group_id, gts_type_id, resource_id)
            .await?
            .ok_or_else(|| {
                DomainError::type_not_found(format!(
                    "Membership ({group_id}, {resource_type}, {resource_id})"
                ))
            })?;

        // Delete the membership
        MembershipRepository::delete(&conn, group_id, gts_type_id, resource_id).await?;
        Ok(())
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-remove:p1:inst-mbr-remove-1

    // @cpt-begin:cpt-cf-resource-group-flow-membership-list:p1:inst-mbr-list-1
    /// List memberships with `OData` filtering and pagination.
    pub async fn list_memberships(
        &self,
        query: &ODataQuery,
    ) -> Result<Page<ResourceGroupMembership>, DomainError> {
        let conn = self.conn()?;
        MembershipRepository::list_memberships(&conn, query).await
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-list:p1:inst-mbr-list-1
}

// -- MembershipAdder trait implementation for seeding --

#[async_trait::async_trait]
impl crate::domain::seeding::MembershipAdder for MembershipService {
    async fn add_membership(
        &self,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), DomainError> {
        self.add_membership(group_id, resource_type, resource_id)
            .await
            .map(|_| ())
    }
}
