# Feature: Group Entity & Hierarchy Engine

- [ ] `p1` - **ID**: `cpt-cf-resource-group-featstatus-entity-hierarchy-implemented`

- [ ] `p1` - `cpt-cf-resource-group-feature-entity-hierarchy`

<!-- toc -->

- [1. Feature Context](#1-feature-context)
  - [1.1 Overview](#11-overview)
  - [1.2 Purpose](#12-purpose)
  - [1.3 Actors](#13-actors)
  - [1.4 References](#14-references)
- [2. Actor Flows (CDSL)](#2-actor-flows-cdsl)
  - [Create Group](#create-group)
  - [Update Group](#update-group)
  - [Move Group (Subtree)](#move-group-subtree)
  - [Delete Group](#delete-group)
- [3. Processes / Business Logic (CDSL)](#3-processes--business-logic-cdsl)
  - [Cycle Detection](#cycle-detection)
  - [Closure Table Rebuild for Subtree Move](#closure-table-rebuild-for-subtree-move)
  - [Query Profile Enforcement](#query-profile-enforcement)
  - [Group Data Seeding](#group-data-seeding)
- [4. States (CDSL)](#4-states-cdsl)
- [5. Definitions of Done](#5-definitions-of-done)
  - [Entity Service](#entity-service)
  - [Hierarchy Engine](#hierarchy-engine)
  - [Group REST Handlers and Hierarchy Endpoint](#group-rest-handlers-and-hierarchy-endpoint)
  - [Group Data Seeding](#group-data-seeding-1)
- [6. Acceptance Criteria](#6-acceptance-criteria)

<!-- /toc -->

## 1. Feature Context

### 1.1 Overview

Group entity lifecycle (create, get, update, move, delete) with strict forest invariants (single parent, no cycles), closure-table-based hierarchy engine for efficient ancestor/descendant queries, query profile enforcement (`max_depth`/`max_width`), subtree move/delete operations, hierarchy depth endpoint with relative depth, force delete, and group data seeding.

### 1.2 Purpose

Groups are the core nodes of the resource group hierarchy. This feature implements the entity service that enforces structural invariants, the hierarchy engine that maintains the closure table projection for efficient graph queries, and the query profile guardrails that bound hierarchy depth and width.

**Requirements**: `cpt-cf-resource-group-fr-manage-entities`, `cpt-cf-resource-group-fr-enforce-forest-hierarchy`, `cpt-cf-resource-group-fr-validate-parent-type`, `cpt-cf-resource-group-fr-delete-entity-no-active-references`, `cpt-cf-resource-group-fr-seed-groups`, `cpt-cf-resource-group-fr-closure-table`, `cpt-cf-resource-group-fr-query-group-hierarchy`, `cpt-cf-resource-group-fr-subtree-operations`, `cpt-cf-resource-group-fr-query-profile`, `cpt-cf-resource-group-fr-profile-change-no-rewrite`, `cpt-cf-resource-group-fr-reduced-constraints-behavior`, `cpt-cf-resource-group-fr-list-groups-depth`, `cpt-cf-resource-group-fr-force-delete`, `cpt-cf-resource-group-nfr-hierarchy-query-latency`

**Principles**: `cpt-cf-resource-group-principle-strict-forest`, `cpt-cf-resource-group-principle-query-profile-guardrail`

**Constraints**: `cpt-cf-resource-group-constraint-profile-change-safety`

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-instance-administrator` | Manages group hierarchy via REST API, operates group seeding |
| `cpt-cf-resource-group-actor-tenant-administrator` | Manages groups within tenant scope via REST API |
| `cpt-cf-resource-group-actor-apps` | Programmatic group management via `ResourceGroupClient` SDK |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md) — sections 5.2, 5.4, 5.5, 5.6, 8.2
- **Design**: [DESIGN.md](../DESIGN.md) — sections 3.1, 3.2 (Entity Service, Hierarchy Service), 3.6 (sequences), 3.7 (resource_group, resource_group_closure), 3.8 (Query Profile)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) entry 2.3
- **Dependencies**: `cpt-cf-resource-group-feature-type-management` (type validation for parent-child compatibility)

## 2. Actor Flows (CDSL)

### Create Group

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-entity-hier-create-group`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Root group created (no parent) with self-referencing closure row
- Child group created under existing parent with full closure path rows

**Error Scenarios**:
- Invalid type → Validation error
- Parent not found → NotFound
- Parent type not in allowed_parents → InvalidParentType
- Type does not allow root placement (can_be_root=false) and no parent → Validation error
- Depth or width limit exceeded → LimitViolation

**Steps**:
1. [ ] - `p1` - Actor sends POST /api/resource-group/v1/groups with {type, name, metadata, hierarchy: {parent_id, tenant_id}} - `inst-create-group-1`
2. [ ] - `p1` - DB: BEGIN transaction (SERIALIZABLE isolation) - `inst-create-group-2`
3. [ ] - `p1` - Resolve type GTS path to surrogate ID; verify type exists - `inst-create-group-3`
4. [ ] - `p1` - **IF** parent_id is provided - `inst-create-group-4`
   1. [ ] - `p1` - DB: SELECT id, gts_type_id, tenant_id FROM resource_group WHERE id = {parent_id} — load parent in tx - `inst-create-group-4a`
   2. [ ] - `p1` - **IF** parent not found → **RETURN** NotFound - `inst-create-group-4b`
   3. [ ] - `p1` - Validate child type's allowed_parents includes parent's type - `inst-create-group-4c`
   4. [ ] - `p1` - **IF** type incompatible → **RETURN** InvalidParentType - `inst-create-group-4d`
   5. [ ] - `p1` - Invoke query profile enforcement: check depth limit - `inst-create-group-4e`
   6. [ ] - `p1` - Invoke query profile enforcement: check width limit (sibling count under parent) - `inst-create-group-4f`
5. [ ] - `p1` - **ELSE** (root group) - `inst-create-group-5`
   1. [ ] - `p1` - **IF** type does not allow root placement (can_be_root=false) → **RETURN** Validation error - `inst-create-group-5a`
6. [ ] - `p1` - DB: INSERT INTO resource_group (id, parent_id, gts_type_id, name, metadata, tenant_id) - `inst-create-group-6`
7. [ ] - `p1` - DB: INSERT INTO resource_group_closure (ancestor_id=id, descendant_id=id, depth=0) — self-row - `inst-create-group-7`
8. [ ] - `p1` - **IF** parent_id is provided - `inst-create-group-8`
   1. [ ] - `p1` - DB: INSERT INTO resource_group_closure — ancestor rows from parent's ancestors with depth+1 - `inst-create-group-8a`
9. [ ] - `p1` - DB: COMMIT - `inst-create-group-9`
10. [ ] - `p1` - **IF** serialization conflict → rollback and retry (bounded retry policy) - `inst-create-group-10`
11. [ ] - `p1` - **RETURN** created ResourceGroup with id, type, name, metadata, hierarchy - `inst-create-group-11`

### Update Group

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-entity-hier-update-group`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Group name, metadata, or type updated successfully
- On type change: children's types validated against new type

**Error Scenarios**:
- Group not found → NotFound
- New type's allowed_parents does not include current parent → InvalidParentType
- Children's types do not include new type in their allowed_parents → InvalidParentType

**Steps**:
1. [ ] - `p1` - Actor sends PUT /api/resource-group/v1/groups/{group_id} with {name, type, metadata} - `inst-update-group-1`
2. [ ] - `p1` - DB: SELECT FROM resource_group WHERE id = {group_id} — load existing group - `inst-update-group-2`
3. [ ] - `p1` - **IF** group not found → **RETURN** NotFound - `inst-update-group-3`
4. [ ] - `p1` - **IF** type is changed - `inst-update-group-4`
   1. [ ] - `p1` - Validate new type's allowed_parents permits current parent's type (or new type allows root if no parent) - `inst-update-group-4a`
   2. [ ] - `p1` - DB: SELECT gts_type_id FROM resource_group WHERE parent_id = {group_id} — load children types - `inst-update-group-4b`
   3. [ ] - `p1` - **FOR EACH** child: verify child's type includes new type in allowed_parents - `inst-update-group-4c`
   4. [ ] - `p1` - **IF** any child would become invalid → **RETURN** InvalidParentType with child details - `inst-update-group-4d`
5. [ ] - `p1` - DB: UPDATE resource_group SET name, gts_type_id, metadata, updated_at — apply changes - `inst-update-group-5`
6. [ ] - `p1` - **RETURN** updated ResourceGroup - `inst-update-group-6`

### Move Group (Subtree)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-entity-hier-move-group`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Group and its entire subtree moved to new parent with closure paths rebuilt transactionally

**Error Scenarios**:
- Group not found → NotFound
- New parent not found → NotFound
- New parent is a descendant of group → CycleDetected
- Self-parent attempt → CycleDetected
- Parent type incompatible → InvalidParentType
- Depth or width limit exceeded at new position → LimitViolation

**Steps**:
1. [ ] - `p1` - Actor sends PUT /api/resource-group/v1/groups/{group_id} with new hierarchy.parent_id - `inst-move-group-1`
2. [ ] - `p1` - DB: BEGIN transaction (SERIALIZABLE isolation) - `inst-move-group-2`
3. [ ] - `p1` - Load group and new parent in transaction - `inst-move-group-3`
4. [ ] - `p1` - **IF** new_parent_id == group_id → **RETURN** CycleDetected (self-parent) - `inst-move-group-4`
5. [ ] - `p1` - Invoke cycle detection: check new parent is NOT in subtree of group - `inst-move-group-5`
6. [ ] - `p1` - **IF** cycle detected → **RETURN** CycleDetected with involved node IDs - `inst-move-group-6`
7. [ ] - `p1` - Validate parent type compatibility for group's type against new parent's type - `inst-move-group-7`
8. [ ] - `p1` - Invoke query profile enforcement: check depth at new position - `inst-move-group-8`
9. [ ] - `p1` - Invoke closure rebuild algorithm for subtree under group - `inst-move-group-9`
10. [ ] - `p1` - DB: UPDATE resource_group SET parent_id = {new_parent_id}, updated_at = now() - `inst-move-group-10`
11. [ ] - `p1` - DB: COMMIT - `inst-move-group-11`
12. [ ] - `p1` - **IF** serialization conflict → rollback and retry (bounded retry policy) - `inst-move-group-12`
13. [ ] - `p1` - **RETURN** updated ResourceGroup - `inst-move-group-13`

### Delete Group

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-entity-hier-delete-group`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Leaf group (no children, no memberships) deleted with closure rows removed
- Force delete: group and entire subtree deleted with all memberships cascaded

**Error Scenarios**:
- Group not found → NotFound
- Has children or memberships (without force) → ConflictActiveReferences

**Steps**:
1. [ ] - `p1` - Actor sends DELETE /api/resource-group/v1/groups/{group_id}?force={true|false} - `inst-delete-group-1`
2. [ ] - `p1` - DB: SELECT FROM resource_group WHERE id = {group_id} - `inst-delete-group-2`
3. [ ] - `p1` - **IF** group not found → **RETURN** NotFound - `inst-delete-group-3`
4. [ ] - `p1` - **IF** force = false - `inst-delete-group-4`
   1. [ ] - `p1` - DB: SELECT COUNT(*) FROM resource_group WHERE parent_id = {group_id} — check children - `inst-delete-group-4a`
   2. [ ] - `p1` - DB: SELECT COUNT(*) FROM resource_group_membership WHERE group_id = {group_id} — check memberships - `inst-delete-group-4b`
   3. [ ] - `p1` - **IF** children > 0 OR memberships > 0 → **RETURN** ConflictActiveReferences - `inst-delete-group-4c`
5. [ ] - `p1` - **IF** force = true - `inst-delete-group-5`
   1. [ ] - `p1` - Collect entire subtree: DB: SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = {group_id} - `inst-delete-group-5a`
   2. [ ] - `p1` - DB: DELETE FROM resource_group_membership WHERE group_id IN (subtree IDs) — cascade memberships - `inst-delete-group-5b`
   3. [ ] - `p1` - DB: DELETE FROM resource_group_closure WHERE ancestor_id IN (subtree IDs) OR descendant_id IN (subtree IDs) — cascade closure - `inst-delete-group-5c`
   4. [ ] - `p1` - DB: DELETE FROM resource_group WHERE id IN (subtree IDs) — delete groups bottom-up - `inst-delete-group-5d`
6. [ ] - `p1` - **ELSE** (leaf delete without force) - `inst-delete-group-6`
   1. [ ] - `p1` - DB: DELETE FROM resource_group_closure WHERE descendant_id = {group_id} — remove closure rows - `inst-delete-group-6a`
   2. [ ] - `p1` - DB: DELETE FROM resource_group WHERE id = {group_id} - `inst-delete-group-6b`
7. [ ] - `p1` - **RETURN** success (204 No Content) - `inst-delete-group-7`

## 3. Processes / Business Logic (CDSL)

### Cycle Detection

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-entity-hier-cycle-detect`

**Input**: Group ID being moved, proposed new parent ID

**Output**: Pass or CycleDetected with involved node IDs

**Steps**:
1. [ ] - `p1` - **IF** new_parent_id == group_id → **RETURN** CycleDetected (self-parent) - `inst-cycle-1`
2. [ ] - `p1` - DB: SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = {group_id} — get all descendants of the moving group - `inst-cycle-2`
3. [ ] - `p1` - **IF** new_parent_id IN descendants → **RETURN** CycleDetected: new parent is a descendant of the moving group - `inst-cycle-3`
4. [ ] - `p1` - **RETURN** pass - `inst-cycle-4`

### Closure Table Rebuild for Subtree Move

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-entity-hier-closure-rebuild`

**Input**: Group ID being moved, old parent ID, new parent ID

**Output**: Updated closure table rows (within active transaction)

**Steps**:
1. [ ] - `p1` - Collect subtree: DB: SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = {group_id} — includes group itself - `inst-closure-rebuild-1`
2. [ ] - `p1` - Delete affected paths: DB: DELETE FROM resource_group_closure WHERE descendant_id IN (subtree) AND ancestor_id NOT IN (subtree) — remove old ancestor paths above the moving group - `inst-closure-rebuild-2`
3. [ ] - `p1` - Compute new ancestor paths from new parent: DB: SELECT ancestor_id, depth FROM resource_group_closure WHERE descendant_id = {new_parent_id} — get new parent's ancestors - `inst-closure-rebuild-3`
4. [ ] - `p1` - **FOR EACH** new_ancestor in new parent's ancestors (including new parent) - `inst-closure-rebuild-4`
   1. [ ] - `p1` - **FOR EACH** subtree_node in subtree - `inst-closure-rebuild-4a`
      1. [ ] - `p1` - DB: INSERT INTO resource_group_closure (ancestor_id = new_ancestor, descendant_id = subtree_node, depth = new_ancestor_depth + subtree_node_relative_depth + 1) - `inst-closure-rebuild-4a1`
5. [ ] - `p1` - **RETURN** (closure rows updated within transaction — commit handled by caller) - `inst-closure-rebuild-5`

### Query Profile Enforcement

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-entity-hier-enforce-query-profile`

**Input**: Operation context (create/move), current group position, profile config (max_depth, max_width)

**Output**: Pass or LimitViolation (DepthLimitExceeded / WidthLimitExceeded)

**Steps**:
1. [ ] - `p1` - Load profile config: max_depth (optional), max_width (optional) - `inst-profile-1`
2. [ ] - `p1` - **IF** max_depth is enabled (not null) - `inst-profile-2`
   1. [ ] - `p1` - Compute resulting depth: depth of new parent + 1 + max descendant depth in subtree (for move) or 0 (for create) - `inst-profile-2a`
   2. [ ] - `p1` - **IF** resulting depth > max_depth → **RETURN** LimitViolation: DepthLimitExceeded with current depth and limit - `inst-profile-2b`
3. [ ] - `p1` - **IF** max_width is enabled (not null) - `inst-profile-3`
   1. [ ] - `p1` - DB: SELECT COUNT(*) FROM resource_group WHERE parent_id = {parent_id} — current sibling count - `inst-profile-3a`
   2. [ ] - `p1` - **IF** sibling_count + 1 > max_width → **RETURN** LimitViolation: WidthLimitExceeded with current width and limit - `inst-profile-3b`
4. [ ] - `p1` - **RETURN** pass - `inst-profile-4`

### Group Data Seeding

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-entity-hier-seed-groups`

**Input**: List of group seed definitions with parent references and type assignments

**Output**: Seed result (groups created, updated, unchanged count)

**Steps**:
1. [ ] - `p1` - Load seed definitions, order by dependency (parents before children) - `inst-seed-groups-1`
2. [ ] - `p1` - **FOR EACH** seed_def in ordered definitions - `inst-seed-groups-2`
   1. [ ] - `p1` - DB: SELECT FROM resource_group WHERE id = {seed_def.id} or name/type match - `inst-seed-groups-2a`
   2. [ ] - `p1` - **IF** group exists AND definition matches → skip (unchanged) - `inst-seed-groups-2b`
   3. [ ] - `p1` - **IF** group exists AND definition differs → update via update flow - `inst-seed-groups-2c`
   4. [ ] - `p1` - **IF** group does not exist → create via create flow (validates type compat, builds closure) - `inst-seed-groups-2d`
3. [ ] - `p1` - **RETURN** seed result: {created: N, updated: N, unchanged: N} - `inst-seed-groups-3`

## 4. States (CDSL)

Not applicable. Groups exist as hierarchy nodes without lifecycle states. A group is either present in the hierarchy or deleted. Structural integrity (parent-child relationships, closure projections) is managed by the entity service and hierarchy engine, not by state machine transitions.

## 5. Definitions of Done

### Entity Service

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-hier-entity-service`

The system **MUST** implement an Entity Service that provides create, get, update, move, and delete operations for group entities with forest invariant enforcement.

**Required behavior**:
- Create: validate type, parent compatibility, profile limits; persist entity + closure rows in SERIALIZABLE tx
- Get: retrieve by UUID; return NotFound if absent
- Update: validate type change against parent and children compatibility; update mutable fields
- Move: cycle detection, parent type validation, profile limits; rebuild closure paths in SERIALIZABLE tx with bounded retry
- Delete: reference check (children + memberships); reject or force-cascade; remove closure rows
- All hierarchy-mutating writes (create/move/delete) use SERIALIZABLE isolation with bounded retry for serialization conflicts

**Implements**:
- `cpt-cf-resource-group-flow-entity-hier-create-group`
- `cpt-cf-resource-group-flow-entity-hier-update-group`
- `cpt-cf-resource-group-flow-entity-hier-move-group`
- `cpt-cf-resource-group-flow-entity-hier-delete-group`
- `cpt-cf-resource-group-algo-entity-hier-cycle-detect`
- `cpt-cf-resource-group-algo-entity-hier-enforce-query-profile`

**Touches**:
- DB: `resource_group`, `resource_group_closure`
- Entities: `ResourceGroupEntity`, `ResourceGroupClosure`

### Hierarchy Engine

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-hier-hierarchy-engine`

The system **MUST** implement a Hierarchy Service that maintains the closure table and serves ancestor/descendant queries.

**Required behavior**:
- Closure table maintenance: self-row on insert, ancestor rows from parent chain, full path rebuild on subtree move, cascade removal on delete
- Ancestor queries: return all ancestors of a group ordered by depth (ascending)
- Descendant queries: return all descendants of a group ordered by depth (ascending)
- Hierarchy depth endpoint: `GET /groups/{group_id}/hierarchy` returning `Page<ResourceGroupWithDepth>` with `hierarchy.depth` (relative: 0=self, positive=descendants, negative=ancestors)
- OData filtering on `hierarchy/depth` (eq, ne, gt, ge, lt, le) and `type` (eq, ne, in)
- Query profile enforcement: `max_depth`/`max_width` checked on writes only; reads return full stored data even if profile was tightened; no data rewrite on profile change

**Implements**:
- `cpt-cf-resource-group-algo-entity-hier-closure-rebuild`
- `cpt-cf-resource-group-algo-entity-hier-enforce-query-profile`

**Constraints**: `cpt-cf-resource-group-constraint-profile-change-safety`

**Touches**:
- DB: `resource_group_closure`, `resource_group`
- Entities: `ResourceGroupClosure`, `ResourceGroupWithDepth`

### Group REST Handlers and Hierarchy Endpoint

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-hier-rest-handlers`

The system **MUST** implement REST endpoint handlers for group management under `/api/resource-group/v1/groups` and the hierarchy depth endpoint.

**Required endpoints**:
- `GET /groups` — list groups with OData `$filter` (fields: `type`, `hierarchy/parent_id`, `id`, `name`; operators: `eq`, `ne`, `in`) and cursor-based pagination
- `POST /groups` — create group, return 201 Created
- `GET /groups/{group_id}` — get group by UUID, return 404 if not found
- `PUT /groups/{group_id}` — update group (name, type, metadata) or move group (hierarchy.parent_id), return 200 OK
- `DELETE /groups/{group_id}?force={true|false}` — delete group, return 204 No Content
- `GET /groups/{group_id}/hierarchy` — hierarchy depth traversal with OData `$filter` on `hierarchy/depth` and `type`, cursor-based pagination

**Implements**:
- `cpt-cf-resource-group-flow-entity-hier-create-group`
- `cpt-cf-resource-group-flow-entity-hier-update-group`
- `cpt-cf-resource-group-flow-entity-hier-move-group`
- `cpt-cf-resource-group-flow-entity-hier-delete-group`

**Touches**:
- API: `GET/POST /api/resource-group/v1/groups`, `GET/PUT/DELETE /api/resource-group/v1/groups/{group_id}`, `GET /api/resource-group/v1/groups/{group_id}/hierarchy`

### Group Data Seeding

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-hier-seeding`

The system **MUST** provide an idempotent group seeding mechanism for deployment bootstrapping.

**Required behavior**:
- Accept ordered list of group seed definitions (parents before children)
- For each seed: create if missing, update if definition differs, skip if unchanged
- Validate parent-child links, type compatibility, and profile limits during seeding
- Seeding runs as a pre-deployment step with system SecurityContext
- Repeated runs produce the same result (idempotent)

**Implements**:
- `cpt-cf-resource-group-algo-entity-hier-seed-groups`

**Touches**:
- DB: `resource_group`, `resource_group_closure`

## 6. Acceptance Criteria

- [ ] Root group (can_be_root=true, no parent) is created with self-referencing closure row (depth=0)
- [ ] Child group is created with closure rows linking to all ancestors at correct depths
- [ ] Creating group with parent of incompatible type returns `InvalidParentType` (409)
- [ ] Creating group with nonexistent parent returns `NotFound` (404)
- [ ] Creating root group when type has can_be_root=false returns validation error (400)
- [ ] Moving group to new parent rebuilds closure paths transactionally for entire subtree
- [ ] Moving group under its own descendant returns `CycleDetected` (409)
- [ ] Moving group under itself (self-parent) returns `CycleDetected` (409)
- [ ] Moving group to incompatible parent type returns `InvalidParentType` (409)
- [ ] Updating group type validates both parent and children compatibility
- [ ] Deleting leaf group (no children, no memberships) succeeds (204) and removes closure rows
- [ ] Deleting group with children without force returns `ConflictActiveReferences` (409)
- [ ] Force delete removes entire subtree including memberships and closure rows
- [ ] Hierarchy endpoint returns ancestors (negative depth) and descendants (positive depth) with correct relative distances
- [ ] OData `$filter` on `hierarchy/depth` supports eq, ne, gt, ge, lt, le operators
- [ ] Write operations that exceed max_depth are rejected with `DepthLimitExceeded`
- [ ] Write operations that exceed max_width are rejected with `WidthLimitExceeded`
- [ ] Reads return full stored data even when profile was tightened (no truncation)
- [ ] Concurrent hierarchy mutations use SERIALIZABLE isolation with bounded retry
- [ ] Group seeding creates hierarchy with correct parent-child links and closure rows (idempotent)
