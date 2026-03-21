//! Idempotent seeding operations for types, groups, and memberships.
//!
//! All seed functions follow the same pattern: for each definition, check if
//! the entity already exists, create if missing, update if the definition
//! differs, and skip if unchanged. Repeated runs produce the same result.

use resource_group_sdk::models::{CreateTypeRequest, UpdateTypeRequest, CreateGroupRequest};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::group_service::GroupService;
use crate::domain::type_service::TypeService;

/// Seed result tracking.
#[derive(Debug, Default)]
pub struct SeedResult {
    /// Number of entities created during this seed run.
    pub created: u32,
    /// Number of entities updated during this seed run.
    pub updated: u32,
    /// Number of entities that already matched the seed definition.
    pub unchanged: u32,
    /// Number of entities skipped due to incompatibility or missing prerequisites.
    pub skipped: u32,
}

// @cpt-algo:cpt-cf-resource-group-algo-type-mgmt-seed-types:p1
// @cpt-dod:cpt-cf-resource-group-dod-type-mgmt-seeding:p1
/// Idempotent type seeding: create if missing, update if differs, skip if unchanged.
pub async fn seed_types(
    type_service: &TypeService,
    seeds: &[CreateTypeRequest],
) -> Result<SeedResult, DomainError> {
    let mut result = SeedResult::default();
    for seed in seeds {
        match type_service.get_type(&seed.code).await {
            Ok(existing) => {
                // Compare: if definition differs, update; otherwise skip
                if existing.can_be_root != seed.can_be_root
                    || existing.allowed_parents != seed.allowed_parents
                    || existing.allowed_memberships != seed.allowed_memberships
                    || existing.metadata_schema != seed.metadata_schema
                {
                    let update_req = UpdateTypeRequest {
                        can_be_root: seed.can_be_root,
                        allowed_parents: seed.allowed_parents.clone(),
                        allowed_memberships: seed.allowed_memberships.clone(),
                        metadata_schema: seed.metadata_schema.clone(),
                    };
                    type_service.update_type(&seed.code, update_req).await?;
                    result.updated += 1;
                } else {
                    result.unchanged += 1;
                }
            }
            Err(DomainError::TypeNotFound { .. }) => {
                type_service.create_type(seed.clone()).await?;
                result.created += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(result)
}

/// Group seed definition with stable identity.
#[derive(Debug, Clone)]
pub struct GroupSeedDef {
    /// Stable identifier for the seeded group.
    pub id: Uuid,
    /// GTS chained type path.
    pub type_path: String,
    /// Display name.
    pub name: String,
    /// Parent group ID (None for root groups).
    pub parent_id: Option<Uuid>,
    /// Type-specific metadata.
    pub metadata: Option<serde_json::Value>,
    /// Tenant scope.
    pub tenant_id: Uuid,
}

// @cpt-algo:cpt-cf-resource-group-algo-entity-hier-seed-groups:p1
// @cpt-dod:cpt-cf-resource-group-dod-entity-hier-seeding:p1
/// Idempotent group seeding: ordered by dependency (parents before children).
///
/// Callers must order `seeds` such that parent groups appear before their
/// children. Each seed is looked up by ID; if the group already exists it is
/// skipped (idempotent), otherwise it is created through the normal service
/// path which enforces type compatibility, tenant scope, and closure table
/// maintenance.
pub async fn seed_groups(
    group_service: &GroupService,
    seeds: &[GroupSeedDef],
) -> Result<SeedResult, DomainError> {
    let mut result = SeedResult::default();
    for seed in seeds {
        let anon = modkit_security::SecurityContext::anonymous();
        match group_service.get_group(&anon, seed.id).await {
            Ok(_existing) => {
                // Group exists -- idempotent skip
                result.unchanged += 1;
            }
            Err(DomainError::GroupNotFound { .. }) => {
                let req = CreateGroupRequest {
                    type_path: seed.type_path.clone(),
                    name: seed.name.clone(),
                    parent_id: seed.parent_id,
                    metadata: seed.metadata.clone(),
                };
                group_service.create_group(req, seed.tenant_id).await?;
                result.created += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(result)
}

/// Membership seed definition.
#[derive(Debug, Clone)]
pub struct MembershipSeedDef {
    /// Target group to add the resource to.
    pub group_id: Uuid,
    /// GTS type path of the resource being linked.
    pub resource_type: String,
    /// Identifier of the resource being linked.
    pub resource_id: String,
}

/// Trait for membership operations required by the seeding function.
///
/// This allows seeding to work with any implementation that can add
/// memberships, decoupling from a concrete `MembershipService`.
#[async_trait::async_trait]
pub trait MembershipAdder: Send + Sync {
    /// Add a membership link. Returns `Ok(())` on success.
    async fn add_membership(
        &self,
        group_id: Uuid,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), DomainError>;
}

// @cpt-algo:cpt-cf-resource-group-algo-membership-seed:p1
// @cpt-dod:cpt-cf-resource-group-dod-membership-seeding:p1
/// Idempotent membership seeding: skip duplicates, validate tenant compat.
///
/// Each seed definition is attempted through the provided adder. Conflicts
/// (duplicate composite keys) are treated as idempotent successes.
/// Tenant-incompatible memberships are logged and skipped rather than
/// failing the entire seed run.
pub async fn seed_memberships(
    adder: &dyn MembershipAdder,
    seeds: &[MembershipSeedDef],
) -> Result<SeedResult, DomainError> {
    let mut result = SeedResult::default();
    for seed in seeds {
        match adder
            .add_membership(seed.group_id, &seed.resource_type, &seed.resource_id)
            .await
        {
            Ok(()) => result.created += 1,
            Err(DomainError::Conflict { .. }) => {
                // Already exists -- idempotent skip
                result.unchanged += 1;
            }
            Err(DomainError::TenantIncompatibility { .. }) => {
                // Tenant mismatch -- skip with warning
                tracing::warn!(
                    group_id = %seed.group_id,
                    resource_type = %seed.resource_type,
                    resource_id = %seed.resource_id,
                    "Skipping membership seed: tenant incompatibility"
                );
                result.skipped += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(result)
}
