use std::sync::Arc;

use modkit_db::secure::DBRunner;
use modkit_security::AccessScope;
use sea_orm::ActiveValue;
use time::OffsetDateTime;
use uuid::Uuid;

use resource_group_sdk::{
    CreateGroupRequest, ListQuery, Page, PageInfo, ResourceGroup, ResourceGroupWithDepth,
    UpdateGroupRequest,
};

use crate::domain::error::DomainError;
use crate::infra::db::entity::{resource_group, resource_group_closure};
use crate::infra::db::repo::closure_repo::ClosureRepository;
use crate::infra::db::repo::group_repo::GroupRepository;
use crate::infra::db::repo::membership_repo::MembershipRepository;
use crate::infra::db::repo::type_repo::TypeRepository;

/// Default page size for list operations.
const DEFAULT_TOP: i32 = 50;
/// Maximum page size for list operations.
const MAX_TOP: i32 = 300;

// @cpt-flow:cpt-cf-resource-group-flow-group-create:p1
// @cpt-flow:cpt-cf-resource-group-flow-group-get:p2
// @cpt-flow:cpt-cf-resource-group-flow-group-list:p2
// @cpt-flow:cpt-cf-resource-group-flow-group-update:p1
// @cpt-flow:cpt-cf-resource-group-flow-group-delete:p1
// @cpt-flow:cpt-cf-resource-group-flow-group-depth:p1
// @cpt-flow:cpt-cf-resource-group-flow-group-seed:p1
// @cpt-algo:cpt-cf-resource-group-algo-parent-type-compat:p1
// @cpt-algo:cpt-cf-resource-group-algo-cycle-detection:p1
// @cpt-algo:cpt-cf-resource-group-algo-closure-recalc:p1
// @cpt-algo:cpt-cf-resource-group-algo-profile-enforcement:p1
// @cpt-algo:cpt-cf-resource-group-algo-force-delete-cascade:p2
// @cpt-req:cpt-cf-resource-group-dod-entity-create:p1
// @cpt-req:cpt-cf-resource-group-dod-entity-read:p1
// @cpt-req:cpt-cf-resource-group-dod-entity-update:p1
// @cpt-req:cpt-cf-resource-group-dod-entity-delete:p1
// @cpt-req:cpt-cf-resource-group-dod-depth-traversal:p1
// @cpt-req:cpt-cf-resource-group-dod-query-profile:p1
// @cpt-req:cpt-cf-resource-group-dod-group-seed:p1

pub struct GroupService<TR, GR, CR, MR> {
    type_repo: TR,
    group_repo: GR,
    closure_repo: CR,
    membership_repo: MR,
    db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
    max_depth: Option<usize>,
    max_width: Option<usize>,
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

fn clamp_top(top: Option<i32>) -> i32 {
    match top {
        Some(t) if t < 1 => DEFAULT_TOP,
        Some(t) if t > MAX_TOP => MAX_TOP,
        Some(t) => t,
        None => DEFAULT_TOP,
    }
}

// ── Standalone algorithm helpers (usable inside transactions) ────────────

// @cpt-begin:cpt-cf-resource-group-algo-parent-type-compat:p1:inst-compat-1
fn check_parent_type_compat(
    child_parents: &[String],
    parent_group_type: Option<&str>,
    child_type_code: &str,
) -> Result<(), DomainError> {
    match parent_group_type {
        // inst-compat-2: root placement
        None => {
            // inst-compat-2a: check if empty string in parents array
            if child_parents.iter().any(String::is_empty) {
                // inst-compat-2a1: compatible
                Ok(())
            } else {
                // inst-compat-2b1: type requires a parent
                Err(DomainError::InvalidParentType {
                    child_type: child_type_code.to_owned(),
                    parent_type: "<root>".to_owned(),
                })
            }
        }
        // inst-compat-3: check parent type in allowed list
        Some(pt) => {
            if child_parents.iter().any(|p| p == pt) {
                // inst-compat-3a: compatible
                Ok(())
            } else {
                // inst-compat-4a: incompatible
                Err(DomainError::InvalidParentType {
                    child_type: child_type_code.to_owned(),
                    parent_type: pt.to_owned(),
                })
            }
        }
    }
}
// @cpt-end:cpt-cf-resource-group-algo-parent-type-compat:p1:inst-compat-1

// @cpt-begin:cpt-cf-resource-group-algo-cycle-detection:p1:inst-cycle-1
async fn check_cycle<C: DBRunner>(
    closure_repo: &impl ClosureRepository,
    conn: &C,
    moving_node_id: Uuid,
    new_parent_id: Uuid,
) -> Result<(), DomainError> {
    // inst-cycle-1: check if moving node is an ancestor of proposed new parent
    let is_ancestor = closure_repo
        .exists_path(conn, moving_node_id, new_parent_id)
        .await?;
    if is_ancestor {
        // inst-cycle-2a: cycle detected
        return Err(DomainError::CycleDetected {
            ancestor_id: moving_node_id,
            descendant_id: new_parent_id,
        });
    }
    // inst-cycle-3a: no cycle
    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-cycle-detection:p1:inst-cycle-1

// @cpt-begin:cpt-cf-resource-group-algo-profile-enforcement:p1:inst-profile-1
async fn enforce_profile<C: DBRunner>(
    max_depth: Option<usize>,
    max_width: Option<usize>,
    closure_repo: &impl ClosureRepository,
    group_repo: &impl GroupRepository,
    conn: &C,
    parent_id: Option<Uuid>,
    scope: &AccessScope,
) -> Result<(), DomainError> {
    // inst-profile-2: check max_depth
    if let Some(max_d) = max_depth {
        let new_depth = if let Some(pid) = parent_id {
            // inst-profile-2a: calculate depth of new node from root
            let ancestors = closure_repo.find_ancestors(conn, pid).await?;
            let parent_depth = ancestors.iter().map(|a| a.depth).max().unwrap_or(0);
            parent_depth + 1
        } else {
            0 // root node
        };
        // inst-profile-2b: check if exceeds limit
        let max_d_i32 = i32::try_from(max_d).unwrap_or(i32::MAX);
        if new_depth > max_d_i32 {
            // inst-profile-2b1: depth limit exceeded
            return Err(DomainError::LimitViolation {
                limit_name: "max_depth".into(),
                current: i64::from(new_depth),
                max: i64::try_from(max_d).unwrap_or(i64::MAX),
            });
        }
    }

    // inst-profile-3: check max_width
    if let (Some(max_w), Some(pid)) = (max_width, parent_id) {
        // inst-profile-3a: count direct children of target parent
        let current_children = group_repo
            .count_children(conn, scope, pid)
            .await?;
        // inst-profile-3b: check if exceeds limit
        let new_width = current_children.saturating_add(1);
        if new_width > max_w as u64 {
            // inst-profile-3b1: width limit exceeded
            return Err(DomainError::LimitViolation {
                limit_name: "max_width".into(),
                current: i64::try_from(new_width).unwrap_or(i64::MAX),
                max: i64::try_from(max_w).unwrap_or(i64::MAX),
            });
        }
    }

    // inst-profile-4: within limits
    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-profile-enforcement:p1:inst-profile-1

// @cpt-begin:cpt-cf-resource-group-algo-closure-recalc:p1:inst-closure-1
async fn recalc_closure<C: DBRunner>(
    closure_repo: &impl ClosureRepository,
    conn: &C,
    moving_node_id: Uuid,
    new_parent_id: Option<Uuid>,
) -> Result<(), DomainError> {
    // inst-closure-1: identify subtree nodes
    let subtree = closure_repo
        .find_descendants(conn, moving_node_id)
        .await?;
    let subtree_ids: Vec<Uuid> = subtree.iter().map(|c| c.descendant_id).collect();

    // inst-closure-2: delete old external ancestor paths
    if !subtree_ids.is_empty() {
        closure_repo
            .delete_external_ancestor_paths(conn, &subtree_ids)
            .await?;
    }

    // inst-closure-3: insert new paths from new parent's ancestors to subtree nodes
    if let Some(new_pid) = new_parent_id {
        let new_ancestors = closure_repo.find_ancestors(conn, new_pid).await?;
        // Re-fetch subtree with internal depths (ancestor_id = moving_node_id)
        let subtree_nodes = closure_repo
            .find_descendants(conn, moving_node_id)
            .await?;

        for ancestor in &new_ancestors {
            for desc in &subtree_nodes {
                let row = resource_group_closure::ActiveModel {
                    ancestor_id: ActiveValue::Set(ancestor.ancestor_id),
                    descendant_id: ActiveValue::Set(desc.descendant_id),
                    depth: ActiveValue::Set(ancestor.depth + desc.depth + 1),
                };
                closure_repo.insert(conn, row).await?;
            }
        }
    }

    // inst-closure-4: recalculation complete
    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-closure-recalc:p1:inst-closure-1

// @cpt-begin:cpt-cf-resource-group-algo-force-delete-cascade:p2:inst-force-1
async fn force_delete_cascade<C: DBRunner>(
    closure_repo: &impl ClosureRepository,
    membership_repo: &impl MembershipRepository,
    group_repo: &impl GroupRepository,
    conn: &C,
    group_id: Uuid,
    scope: &AccessScope,
) -> Result<(), DomainError> {
    // inst-force-1: get subtree nodes, deepest first
    let mut subtree = closure_repo.find_descendants(conn, group_id).await?;
    subtree.sort_by(|a, b| b.depth.cmp(&a.depth)); // deepest first

    // inst-force-2: for each descendant
    for node in subtree {
        let node_id = node.descendant_id;
        // inst-force-2a: delete all memberships
        membership_repo
            .delete_all_by_group(conn, node_id)
            .await?;
        // inst-force-2b: delete all closure rows
        closure_repo.delete_all_for_node(conn, node_id).await?;
        // inst-force-2c: delete the group
        group_repo.delete(conn, scope, node_id).await?;
    }

    // inst-force-3: cascade complete
    Ok(())
}
// @cpt-end:cpt-cf-resource-group-algo-force-delete-cascade:p2:inst-force-1

// ── GroupService implementation ──────────────────────────────────────────

#[allow(clippy::missing_errors_doc)]
impl<TR, GR, CR, MR> GroupService<TR, GR, CR, MR>
where
    TR: TypeRepository + Clone + Send + Sync + 'static,
    GR: GroupRepository + Clone + Send + Sync + 'static,
    CR: ClosureRepository + Clone + Send + Sync + 'static,
    MR: MembershipRepository + Clone + Send + Sync + 'static,
{
    pub fn new(
        type_repo: TR,
        group_repo: GR,
        closure_repo: CR,
        membership_repo: MR,
        db: Arc<modkit_db::DBProvider<modkit_db::DbError>>,
        max_depth: Option<usize>,
        max_width: Option<usize>,
    ) -> Self {
        Self {
            type_repo,
            group_repo,
            closure_repo,
            membership_repo,
            db,
            max_depth,
            max_width,
        }
    }

    fn conn(&self) -> Result<impl DBRunner + '_, DomainError> {
        self.db
            .conn()
            .map_err(|e| DomainError::database(e.to_string()))
    }

    // @cpt-begin:cpt-cf-resource-group-flow-group-create:p1:inst-grp-create-1
    pub async fn create_group(
        &self,
        request: CreateGroupRequest,
        scope: &AccessScope,
    ) -> Result<ResourceGroup, DomainError> {
        let type_repo = self.type_repo.clone();
        let group_repo = self.group_repo.clone();
        let closure_repo = self.closure_repo.clone();
        let max_depth = self.max_depth;
        let max_width = self.max_width;
        let scope = scope.clone();

        let group_type = request.group_type;
        let name = request.name;
        let parent_id = request.parent_id;
        let tenant_id = request.tenant_id;
        let external_id = request.external_id;

        // inst-grp-create-2: transaction
        let model = self
            .db
            .transaction(|tx| {
                Box::pin(async move {
                    let d = domain_to_db;

                    // inst-grp-create-3: load type definition
                    let type_model = type_repo
                        .find_by_code(tx, &group_type)
                        .await
                        .map_err(d)?
                        .ok_or_else(|| {
                            // inst-grp-create-4b: type not found
                            d(DomainError::TypeNotFound {
                                code: group_type.clone(),
                            })
                        })?;

                    // inst-grp-create-5/6: validate parent type compatibility
                    if let Some(pid) = parent_id {
                        // inst-grp-create-5a: load parent
                        let parent = group_repo
                            .find_by_id(tx, &scope, pid)
                            .await
                            .map_err(d)?
                            .ok_or_else(|| {
                                // inst-grp-create-5b2: parent not found
                                d(DomainError::GroupNotFound { id: pid })
                            })?;

                        // inst-grp-create-5c: parent type compatibility
                        check_parent_type_compat(
                            &type_model.parents_vec(),
                            Some(&parent.group_type),
                            &group_type,
                        )
                        .map_err(d)?;

                        // inst-grp-create-7: profile enforcement
                        enforce_profile(
                            max_depth,
                            max_width,
                            &closure_repo,
                            &group_repo,
                            tx,
                            Some(pid),
                            &scope,
                        )
                        .await
                        .map_err(d)?;
                    } else {
                        // inst-grp-create-6a: verify root placement allowed
                        check_parent_type_compat(
                            &type_model.parents_vec(),
                            None,
                            &group_type,
                        )
                        .map_err(d)?;
                    }

                    // inst-grp-create-9: insert group entity
                    let new_id = Uuid::new_v4();
                    let now = OffsetDateTime::now_utc();
                    let active = resource_group::ActiveModel {
                        id: ActiveValue::Set(new_id),
                        parent_id: ActiveValue::Set(parent_id),
                        group_type: ActiveValue::Set(group_type),
                        name: ActiveValue::Set(name),
                        tenant_id: ActiveValue::Set(tenant_id),
                        external_id: ActiveValue::Set(external_id),
                        created: ActiveValue::Set(now),
                        modified: ActiveValue::NotSet,
                    };
                    let model = group_repo
                        .insert(tx, &scope, active)
                        .await
                        .map_err(d)?;

                    // inst-grp-create-10: insert self-row closure
                    let self_closure = resource_group_closure::ActiveModel {
                        ancestor_id: ActiveValue::Set(new_id),
                        descendant_id: ActiveValue::Set(new_id),
                        depth: ActiveValue::Set(0),
                    };
                    closure_repo.insert(tx, self_closure).await.map_err(d)?;

                    // inst-grp-create-11: insert ancestor-descendant closure rows
                    if let Some(pid) = parent_id {
                        // inst-grp-create-11a: insert paths from parent's ancestors
                        let ancestors =
                            closure_repo.find_ancestors(tx, pid).await.map_err(d)?;
                        for anc in ancestors {
                            let row = resource_group_closure::ActiveModel {
                                ancestor_id: ActiveValue::Set(anc.ancestor_id),
                                descendant_id: ActiveValue::Set(new_id),
                                depth: ActiveValue::Set(anc.depth + 1),
                            };
                            closure_repo.insert(tx, row).await.map_err(d)?;
                        }
                    }

                    // inst-grp-create-12: commit (handled by transaction)
                    // inst-grp-create-14: return created group
                    Ok(model)
                })
            })
            .await
            .map_err(unwrap_domain)?;

        Ok(to_sdk_group(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-create:p1:inst-grp-create-1

    // @cpt-begin:cpt-cf-resource-group-flow-group-get:p2:inst-grp-get-1
    pub async fn get_group(
        &self,
        group_id: Uuid,
        scope: &AccessScope,
    ) -> Result<ResourceGroup, DomainError> {
        let conn = self.conn()?;

        // inst-grp-get-2: select by id
        let model = self
            .group_repo
            .find_by_id(&conn, scope, group_id)
            .await?
            // inst-grp-get-4a: not found
            .ok_or(DomainError::GroupNotFound { id: group_id })?;

        // inst-grp-get-3a: return group
        Ok(to_sdk_group(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-get:p2:inst-grp-get-1

    // @cpt-begin:cpt-cf-resource-group-flow-group-list:p2:inst-grp-list-1
    pub async fn list_groups(
        &self,
        query: ListQuery,
        scope: &AccessScope,
    ) -> Result<Page<ResourceGroup>, DomainError> {
        let conn = self.conn()?;

        let top = clamp_top(query.top);
        let skip = query.skip.unwrap_or(0).max(0);

        // inst-grp-list-4: query with filter, order, pagination
        let models = self
            .group_repo
            .list_filtered(&conn, scope, query.filter.as_deref(), top, skip)
            .await?;

        // inst-grp-list-5: return page
        let items = models.into_iter().map(|m| to_sdk_group(&m)).collect();
        Ok(Page {
            items,
            page_info: PageInfo { top, skip },
        })
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-list:p2:inst-grp-list-1

    // @cpt-begin:cpt-cf-resource-group-flow-group-update:p1:inst-grp-update-1
    pub async fn update_group(
        &self,
        group_id: Uuid,
        request: UpdateGroupRequest,
        scope: &AccessScope,
    ) -> Result<ResourceGroup, DomainError> {
        let type_repo = self.type_repo.clone();
        let group_repo = self.group_repo.clone();
        let closure_repo = self.closure_repo.clone();
        let max_depth = self.max_depth;
        let max_width = self.max_width;
        let scope = scope.clone();

        let new_group_type = request.group_type;
        let new_name = request.name;
        let new_parent_id = request.parent_id;
        let new_external_id = request.external_id;

        // inst-grp-update-2: transaction
        let model = self
            .db
            .transaction(|tx| {
                Box::pin(async move {
                    let d = domain_to_db;

                    // inst-grp-update-3: load current group
                    let current = group_repo
                        .find_by_id(tx, &scope, group_id)
                        .await
                        .map_err(d)?
                        .ok_or_else(|| {
                            // inst-grp-update-4b: not found
                            d(DomainError::GroupNotFound { id: group_id })
                        })?;

                    let type_changed = current.group_type != new_group_type;
                    let parent_changed = current.parent_id != new_parent_id;

                    // Determine the effective parent type for type compatibility check
                    let effective_parent_id = if parent_changed {
                        new_parent_id
                    } else {
                        current.parent_id
                    };

                    // inst-grp-update-5: if group_type changed, validate with current/new parent
                    if type_changed || parent_changed {
                        let type_model = type_repo
                            .find_by_code(tx, &new_group_type)
                            .await
                            .map_err(d)?
                            .ok_or_else(|| {
                                d(DomainError::TypeNotFound {
                                    code: new_group_type.clone(),
                                })
                            })?;

                        if let Some(pid) = effective_parent_id {
                            // inst-grp-update-6a1: load new/current parent
                            let parent = group_repo
                                .find_by_id(tx, &scope, pid)
                                .await
                                .map_err(d)?
                                .ok_or_else(|| {
                                    // inst-grp-update-6a2: parent not found
                                    d(DomainError::GroupNotFound { id: pid })
                                })?;

                            // inst-grp-update-6a3: type compatibility
                            check_parent_type_compat(
                                &type_model.parents_vec(),
                                Some(&parent.group_type),
                                &new_group_type,
                            )
                            .map_err(d)?;
                        } else {
                            // Moving to root — check root placement
                            check_parent_type_compat(
                                &type_model.parents_vec(),
                                None,
                                &new_group_type,
                            )
                            .map_err(d)?;
                        }
                    }

                    // inst-grp-update-6: if parent_id changed (move operation)
                    if parent_changed {
                        if let Some(new_pid) = new_parent_id {
                            // inst-grp-update-6b: cycle detection
                            check_cycle(&closure_repo, tx, group_id, new_pid)
                                .await
                                .map_err(d)?;

                            // inst-grp-update-6d: profile enforcement for new position
                            enforce_profile(
                                max_depth,
                                max_width,
                                &closure_repo,
                                &group_repo,
                                tx,
                                Some(new_pid),
                                &scope,
                            )
                            .await
                            .map_err(d)?;
                        }

                        // inst-grp-update-6f: closure recalculation
                        recalc_closure(&closure_repo, tx, group_id, new_parent_id)
                            .await
                            .map_err(d)?;
                    }

                    // inst-grp-update-7: update group fields
                    let active = resource_group::ActiveModel {
                        id: ActiveValue::Unchanged(group_id),
                        parent_id: ActiveValue::Set(new_parent_id),
                        group_type: ActiveValue::Set(new_group_type),
                        name: ActiveValue::Set(new_name),
                        tenant_id: ActiveValue::NotSet,
                        external_id: ActiveValue::Set(new_external_id),
                        created: ActiveValue::NotSet,
                        modified: ActiveValue::Set(Some(OffsetDateTime::now_utc())),
                    };
                    let updated = group_repo
                        .update(tx, &scope, group_id, active)
                        .await
                        .map_err(d)?;

                    // inst-grp-update-8: commit (handled by transaction)
                    // inst-grp-update-10: return updated group
                    Ok(updated)
                })
            })
            .await
            .map_err(unwrap_domain)?;

        Ok(to_sdk_group(&model))
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-update:p1:inst-grp-update-1

    // @cpt-begin:cpt-cf-resource-group-flow-group-delete:p1:inst-grp-delete-1
    pub async fn delete_group(
        &self,
        group_id: Uuid,
        force: bool,
        scope: &AccessScope,
    ) -> Result<(), DomainError> {
        let group_repo = self.group_repo.clone();
        let closure_repo = self.closure_repo.clone();
        let membership_repo = self.membership_repo.clone();
        let scope = scope.clone();

        // inst-grp-delete-2: transaction
        self.db
            .transaction(|tx| {
                Box::pin(async move {
                    let d = domain_to_db;

                    // inst-grp-delete-3: verify existence
                    group_repo
                        .find_by_id(tx, &scope, group_id)
                        .await
                        .map_err(d)?
                        .ok_or_else(|| {
                            // inst-grp-delete-4b: not found
                            d(DomainError::GroupNotFound { id: group_id })
                        })?;

                    if force {
                        // inst-grp-delete-6a: force delete cascade
                        force_delete_cascade(
                            &closure_repo,
                            &membership_repo,
                            &group_repo,
                            tx,
                            group_id,
                            &scope,
                        )
                        .await
                        .map_err(d)?;
                    } else {
                        // inst-grp-delete-5a: count children
                        let children = group_repo
                            .count_children(tx, &scope, group_id)
                            .await
                            .map_err(d)?;
                        // inst-grp-delete-5b: count memberships
                        let memberships = membership_repo
                            .count_by_group(tx, group_id)
                            .await
                            .map_err(d)?;

                        // inst-grp-delete-5c: reject if active references
                        #[allow(clippy::cast_possible_wrap)]
                        let ref_count =
                            i64::try_from(children + memberships).unwrap_or(i64::MAX);
                        if children > 0 || memberships > 0 {
                            // inst-grp-delete-5c2: conflict
                            return Err(d(DomainError::ActiveReferences {
                                count: ref_count,
                            }));
                        }

                        // inst-grp-delete-5d: remove closure rows
                        closure_repo
                            .delete_all_for_node(tx, group_id)
                            .await
                            .map_err(d)?;

                        // inst-grp-delete-5e: delete group
                        group_repo
                            .delete(tx, &scope, group_id)
                            .await
                            .map_err(d)?;
                    }

                    // inst-grp-delete-7: commit (handled by transaction)
                    // inst-grp-delete-9: success
                    Ok(())
                })
            })
            .await
            .map_err(unwrap_domain)
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-delete:p1:inst-grp-delete-1

    // @cpt-begin:cpt-cf-resource-group-flow-group-depth:p1:inst-grp-depth-1
    pub async fn list_group_depth(
        &self,
        group_id: Uuid,
        query: ListQuery,
        scope: &AccessScope,
    ) -> Result<Page<ResourceGroupWithDepth>, DomainError> {
        let conn = self.conn()?;

        // inst-grp-depth-2: verify reference group exists
        self.group_repo
            .find_by_id(&conn, scope, group_id)
            .await?
            .ok_or(DomainError::GroupNotFound { id: group_id })?;

        // inst-grp-depth-6: get descendants with positive depth
        let descendants = self
            .closure_repo
            .find_descendants(&conn, group_id)
            .await?;

        // inst-grp-depth-7: get ancestors with negative depth
        let ancestors = self
            .closure_repo
            .find_ancestors(&conn, group_id)
            .await?;

        // inst-grp-depth-8: merge results with relative depth
        let mut entries: Vec<(Uuid, i32)> = Vec::new();

        // Descendants: depth is positive (includes self at depth=0)
        for d in &descendants {
            entries.push((d.descendant_id, d.depth));
        }

        // Ancestors: depth is negated (skip self-row already added from descendants)
        for a in &ancestors {
            if a.depth > 0 {
                entries.push((a.ancestor_id, -a.depth));
            }
        }

        // Sort by (depth ASC, group_id ASC)
        entries.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        // inst-grp-depth-9: apply pagination
        let top = clamp_top(query.top);
        let skip = query.skip.unwrap_or(0).max(0);

        let skip_usize = usize::try_from(skip).unwrap_or(0);
        let top_usize = usize::try_from(top).unwrap_or(50);

        let page_entries: Vec<(Uuid, i32)> = entries
            .into_iter()
            .skip(skip_usize)
            .take(top_usize)
            .collect();

        // Fetch group details for page entries
        let mut items = Vec::with_capacity(page_entries.len());
        for (gid, depth) in page_entries {
            if let Some(model) = self
                .group_repo
                .find_by_id(&conn, scope, gid)
                .await?
            {
                items.push(to_sdk_group_with_depth(&model, depth));
            }
        }

        // inst-grp-depth-10: return page
        Ok(Page {
            items,
            page_info: PageInfo { top, skip },
        })
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-depth:p1:inst-grp-depth-1

    // @cpt-begin:cpt-cf-resource-group-flow-group-seed:p1:inst-grp-seed-1
    pub async fn seed_groups(
        &self,
        groups: Vec<CreateGroupRequest>,
    ) -> Result<(), DomainError> {
        let seed_scope = AccessScope::allow_all();

        // inst-grp-seed-2: for each group definition (ordered by dependency)
        for group_def in groups {
            let conn = self.conn()?;

            // inst-grp-seed-2a: validate type exists
            let type_model = self
                .type_repo
                .find_by_code(&conn, &group_def.group_type)
                .await?
                .ok_or_else(|| DomainError::TypeNotFound {
                    // inst-grp-seed-2b: type not found
                    code: group_def.group_type.clone(),
                })?;

            // inst-grp-seed-2c: validate parent compatibility
            if let Some(pid) = group_def.parent_id {
                let parent = self
                    .group_repo
                    .find_by_id(&conn, &seed_scope, pid)
                    .await?
                    .ok_or(DomainError::GroupNotFound { id: pid })?;

                check_parent_type_compat(
                    &type_model.parents_vec(),
                    Some(&parent.group_type),
                    &group_def.group_type,
                )?;
            } else {
                check_parent_type_compat(
                    &type_model.parents_vec(),
                    None,
                    &group_def.group_type,
                )?;
            }

            // inst-grp-seed-2d: upsert group
            let existing = self
                .group_repo
                .find_by_id(&conn, &seed_scope, group_def.tenant_id)
                .await?;

            if existing.is_none() {
                // Create new group via the standard create flow
                self.create_group(group_def, &seed_scope).await?;
            }
        }

        // inst-grp-seed-3: complete
        Ok(())
    }
    // @cpt-end:cpt-cf-resource-group-flow-group-seed:p1:inst-grp-seed-1
}

// ── Conversion helpers ───────────────────────────────────────────────────

fn to_sdk_group(model: &resource_group::Model) -> ResourceGroup {
    ResourceGroup {
        group_id: model.id,
        parent_id: model.parent_id,
        group_type: model.group_type.clone(),
        name: model.name.clone(),
        tenant_id: model.tenant_id,
        external_id: model.external_id.clone(),
    }
}

fn to_sdk_group_with_depth(model: &resource_group::Model, depth: i32) -> ResourceGroupWithDepth {
    ResourceGroupWithDepth {
        group_id: model.id,
        parent_id: model.parent_id,
        group_type: model.group_type.clone(),
        name: model.name.clone(),
        tenant_id: model.tenant_id,
        external_id: model.external_id.clone(),
        depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_type_compat_allows_root_when_empty_string_in_parents() {
        let parents = vec![String::new(), "org".to_owned()];
        assert!(check_parent_type_compat(&parents, None, "tenant").is_ok());
    }

    #[test]
    fn parent_type_compat_rejects_root_when_no_empty_string() {
        let parents = vec!["org".to_owned()];
        assert!(check_parent_type_compat(&parents, None, "dept").is_err());
    }

    #[test]
    fn parent_type_compat_allows_matching_parent_type() {
        let parents = vec!["org".to_owned(), "tenant".to_owned()];
        assert!(check_parent_type_compat(&parents, Some("org"), "dept").is_ok());
        assert!(check_parent_type_compat(&parents, Some("tenant"), "dept").is_ok());
    }

    #[test]
    fn parent_type_compat_rejects_non_matching_parent_type() {
        let parents = vec!["org".to_owned()];
        assert!(check_parent_type_compat(&parents, Some("tenant"), "dept").is_err());
    }

    #[test]
    fn clamp_top_defaults_and_bounds() {
        assert_eq!(clamp_top(None), DEFAULT_TOP);
        assert_eq!(clamp_top(Some(0)), DEFAULT_TOP);
        assert_eq!(clamp_top(Some(-5)), DEFAULT_TOP);
        assert_eq!(clamp_top(Some(10)), 10);
        assert_eq!(clamp_top(Some(300)), 300);
        assert_eq!(clamp_top(Some(500)), MAX_TOP);
    }
}
