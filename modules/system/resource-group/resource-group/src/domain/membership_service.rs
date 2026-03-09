use std::sync::Arc;

use modkit_db::secure::DBRunner;
use modkit_security::AccessScope;
use sea_orm::ActiveValue;
use time::OffsetDateTime;
use uuid::Uuid;

use resource_group_sdk::{
    AddMembershipRequest, ListQuery, Page, PageInfo, RemoveMembershipRequest,
    ResourceGroupMembership,
};

use crate::domain::error::DomainError;
use crate::infra::db::repo::group_repo::GroupRepository;
use crate::infra::db::repo::membership_repo::MembershipRepository;

/// Default page size for list operations.
const DEFAULT_TOP: i32 = 50;
/// Maximum page size for list operations.
const MAX_TOP: i32 = 300;

// @cpt-flow:cpt-cf-resource-group-flow-membership-add:p1
// @cpt-flow:cpt-cf-resource-group-flow-membership-remove:p1
// @cpt-flow:cpt-cf-resource-group-flow-membership-list:p1
// @cpt-flow:cpt-cf-resource-group-flow-membership-seed:p1
// @cpt-algo:cpt-cf-resource-group-algo-membership-tenant-scope:p1
// @cpt-req:cpt-cf-resource-group-dod-membership-add:p1
// @cpt-req:cpt-cf-resource-group-dod-membership-remove:p1
// @cpt-req:cpt-cf-resource-group-dod-membership-list:p1
// @cpt-req:cpt-cf-resource-group-dod-membership-active-ref-guard:p1
// @cpt-req:cpt-cf-resource-group-dod-membership-seed:p1

pub struct MembershipService<GR, MR> {
    group_repo: GR,
    membership_repo: MR,
    db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
}

// ── Transaction error helpers ────────────────────────────────────────────

fn domain_to_db(e: DomainError) -> modkit_db::DbError {
    modkit_db::DbError::Other(anyhow::Error::new(e))
}

fn unwrap_domain(e: modkit_db::DbError) -> DomainError {
    match e {
        modkit_db::DbError::Other(err) => match err.downcast::<DomainError>() {
            Ok(de) => de,
            Err(other) => DomainError::database(other.to_string()),
        },
        other => DomainError::from(other),
    }
}

fn scope() -> AccessScope {
    AccessScope::allow_all()
}

fn clamp_top(top: Option<i32>) -> i32 {
    match top {
        Some(t) if t < 1 => DEFAULT_TOP,
        Some(t) if t > MAX_TOP => MAX_TOP,
        Some(t) => t,
        None => DEFAULT_TOP,
    }
}

// ── Tenant scope validation algorithm ─────────────────────────────────────

// @cpt-begin:cpt-cf-resource-group-algo-membership-tenant-scope:p1:inst-tenant-1
/// Validate that the caller's tenant scope is compatible with the target group's tenant.
///
/// In the current implementation, the ownership-graph profile is not active,
/// so tenant scope validation always returns compatible (no enforcement).
/// When ownership-graph profile is enabled, this function will enforce:
/// - Platform-admin bypasses tenant scope check
/// - Caller's effective tenant scope must cover the target group's `tenant_id`
fn check_tenant_scope(
    _caller_tenant_id: Option<Uuid>,
    _target_tenant_id: Uuid,
    _is_platform_admin: bool,
) {
    // inst-tenant-1: ownership-graph profile is not active — return compatible
}
// @cpt-end:cpt-cf-resource-group-algo-membership-tenant-scope:p1:inst-tenant-1

// ── MembershipService implementation ──────────────────────────────────────

#[allow(clippy::missing_errors_doc)]
impl<GR, MR> MembershipService<GR, MR>
where
    GR: GroupRepository + Clone + Send + Sync + 'static,
    MR: MembershipRepository + Clone + Send + Sync + 'static,
{
    pub fn new(
        group_repo: GR,
        membership_repo: MR,
        db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
    ) -> Self {
        Self {
            group_repo,
            membership_repo,
            db,
        }
    }

    fn conn(&self) -> Result<impl DBRunner + '_, DomainError> {
        self.db
            .conn()
            .map_err(|e| DomainError::database(e.to_string()))
    }

    // @cpt-begin:cpt-cf-resource-group-flow-membership-add:p1:inst-mbr-add-1
    pub async fn add_membership(
        &self,
        request: AddMembershipRequest,
    ) -> Result<ResourceGroupMembership, DomainError> {
        let group_repo = self.group_repo.clone();
        let membership_repo = self.membership_repo.clone();

        let group_id = request.group_id;
        let resource_type = request.resource_type;
        let resource_id = request.resource_id;

        // inst-mbr-add-2..8: transaction
        let model = self
            .db
            .transaction(|tx| {
                Box::pin(async move {
                    let d = domain_to_db;

                    // inst-mbr-add-2: verify group exists and load tenant_id
                    let group = group_repo
                        .find_by_id(tx, &scope(), group_id)
                        .await
                        .map_err(d)?
                        .ok_or_else(|| {
                            // inst-mbr-add-3a: group not found
                            d(DomainError::GroupNotFound { id: group_id })
                        })?;

                    // inst-mbr-add-4: tenant scope validation
                    check_tenant_scope(None, group.tenant_id, false);

                    // inst-mbr-add-6: check for duplicate before insert
                    let existing = membership_repo
                        .find_by_key(tx, group_id, &resource_type, &resource_id)
                        .await
                        .map_err(d)?;

                    if existing.is_some() {
                        // inst-mbr-add-7a: duplicate — conflict
                        return Err(d(DomainError::ActiveReferences { count: 1 }));
                    }

                    // inst-mbr-add-6: insert membership
                    let active =
                        crate::infra::db::entity::resource_group_membership::ActiveModel {
                            group_id: ActiveValue::Set(group_id),
                            resource_type: ActiveValue::Set(resource_type),
                            resource_id: ActiveValue::Set(resource_id),
                            created: ActiveValue::Set(OffsetDateTime::now_utc()),
                        };
                    let model = membership_repo.insert(tx, active).await.map_err(d)?;

                    // inst-mbr-add-8: return created membership
                    Ok(model)
                })
            })
            .await
            .map_err(unwrap_domain)?;

        Ok(to_sdk_membership(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-add:p1:inst-mbr-add-1

    // @cpt-begin:cpt-cf-resource-group-flow-membership-remove:p1:inst-mbr-remove-1
    pub async fn remove_membership(
        &self,
        request: RemoveMembershipRequest,
    ) -> Result<(), DomainError> {
        let group_repo = self.group_repo.clone();
        let membership_repo = self.membership_repo.clone();

        let group_id = request.group_id;
        let resource_type = request.resource_type;
        let resource_id = request.resource_id;

        // inst-mbr-remove-2..8: transaction
        self.db
            .transaction(|tx| {
                Box::pin(async move {
                    let d = domain_to_db;

                    // inst-mbr-remove-2: verify membership exists
                    membership_repo
                        .find_by_key(tx, group_id, &resource_type, &resource_id)
                        .await
                        .map_err(d)?
                        .ok_or_else(|| {
                            // inst-mbr-remove-3a: membership not found
                            d(DomainError::MembershipNotFound {
                                group_id,
                                resource_type: resource_type.clone(),
                                resource_id: resource_id.clone(),
                            })
                        })?;

                    // inst-mbr-remove-4: load group tenant for scope check
                    let group = group_repo
                        .find_by_id(tx, &scope(), group_id)
                        .await
                        .map_err(d)?
                        .ok_or_else(|| d(DomainError::GroupNotFound { id: group_id }))?;

                    // inst-mbr-remove-5: tenant scope validation
                    check_tenant_scope(None, group.tenant_id, false);

                    // inst-mbr-remove-7: delete membership
                    membership_repo
                        .delete(tx, group_id, &resource_type, &resource_id)
                        .await
                        .map_err(d)?;

                    // inst-mbr-remove-8: success
                    Ok(())
                })
            })
            .await
            .map_err(unwrap_domain)
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-remove:p1:inst-mbr-remove-1

    // @cpt-begin:cpt-cf-resource-group-flow-membership-list:p1:inst-mbr-list-1
    pub async fn list_memberships(
        &self,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, DomainError> {
        let conn = self.conn()?;

        let top = clamp_top(query.top);
        let skip = query.skip.unwrap_or(0).max(0);

        // inst-mbr-list-4: query with filter, order, pagination
        let models = self
            .membership_repo
            .list_filtered(&conn, query.filter.as_deref(), top, skip)
            .await?;

        // inst-mbr-list-5: return page
        let items = models.iter().map(to_sdk_membership).collect();
        Ok(Page {
            items,
            page_info: PageInfo { top, skip },
        })
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-list:p1:inst-mbr-list-1

    // @cpt-begin:cpt-cf-resource-group-flow-membership-seed:p1:inst-mbr-seed-1
    pub async fn seed_memberships(
        &self,
        memberships: Vec<AddMembershipRequest>,
    ) -> Result<(), DomainError> {
        // inst-mbr-seed-2: for each membership definition
        for mbr_def in memberships {
            let conn = self.conn()?;

            // inst-mbr-seed-2a: verify group exists
            self.group_repo
                .find_by_id(&conn, &scope(), mbr_def.group_id)
                .await?
                .ok_or(
                    // inst-mbr-seed-2b: group not found — abort
                    DomainError::GroupNotFound {
                        id: mbr_def.group_id,
                    },
                )?;

            // inst-mbr-seed-2c: idempotent insert (check existence first)
            let existing = self
                .membership_repo
                .find_by_key(
                    &conn,
                    mbr_def.group_id,
                    &mbr_def.resource_type,
                    &mbr_def.resource_id,
                )
                .await?;

            if existing.is_none() {
                let active =
                    crate::infra::db::entity::resource_group_membership::ActiveModel {
                        group_id: ActiveValue::Set(mbr_def.group_id),
                        resource_type: ActiveValue::Set(mbr_def.resource_type),
                        resource_id: ActiveValue::Set(mbr_def.resource_id),
                        created: ActiveValue::Set(OffsetDateTime::now_utc()),
                    };
                self.membership_repo.insert(&conn, active).await?;
            }
        }

        // inst-mbr-seed-3: complete
        Ok(())
    }
    // @cpt-end:cpt-cf-resource-group-flow-membership-seed:p1:inst-mbr-seed-1
}

// ── Conversion helpers ───────────────────────────────────────────────────

fn to_sdk_membership(
    model: &crate::infra::db::entity::resource_group_membership::Model,
) -> ResourceGroupMembership {
    ResourceGroupMembership {
        group_id: model.group_id,
        resource_type: model.resource_type.clone(),
        resource_id: model.resource_id.clone(),
    }
}
