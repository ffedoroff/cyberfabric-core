# Feature: Entity & Hierarchy Management

- [ ] `p1` - **ID**: `cpt-cf-resource-group-featstatus-entity-hierarchy`
- [ ] `p1` - `cpt-cf-resource-group-feature-entity-hierarchy`

## 1. Feature Context

### 1.1 Overview

Implement resource group entity CRUD with strict forest topology enforcement, closure-table hierarchy maintenance, subtree operations (move and force delete), depth-based hierarchy queries with relative depth, query profile enforcement (`max_depth`/`max_width` guardrails), and deterministic seed path for pre-deployment hierarchy provisioning. All hierarchy-mutating writes execute inside SERIALIZABLE transactions with bounded retry on serialization conflicts.

### 1.2 Purpose

Groups are the core structural element of the RG hierarchy. Without entity and hierarchy management, memberships (Feature 4) and integration reads (Feature 5) have no data to operate on. This feature establishes the forest topology, closure-table read model, and all write invariants that downstream features depend on.

Addresses:
- `cpt-cf-resource-group-fr-manage-entities` — entity CRUD
- `cpt-cf-resource-group-fr-enforce-forest-hierarchy` — single parent, cycle prevention
- `cpt-cf-resource-group-fr-validate-parent-type` — parent-child type compatibility
- `cpt-cf-resource-group-fr-delete-entity-no-active-references` — delete guard
- `cpt-cf-resource-group-fr-closure-table` — closure-table maintenance
- `cpt-cf-resource-group-fr-query-group-hierarchy` — ancestor/descendant queries
- `cpt-cf-resource-group-fr-subtree-operations` — subtree move/delete
- `cpt-cf-resource-group-fr-query-profile` — depth/width guardrails
- `cpt-cf-resource-group-fr-profile-change-no-rewrite` — tightened profiles never rewrite rows
- `cpt-cf-resource-group-fr-reduced-constraints-behavior` — tightened profiles reject violating writes
- `cpt-cf-resource-group-fr-force-delete` — cascade subtree and memberships
- `cpt-cf-resource-group-fr-list-groups-depth` — depth endpoint with relative depth
- `cpt-cf-resource-group-fr-seed-groups` — deterministic hierarchy seeding
- `cpt-cf-resource-group-nfr-hierarchy-query-latency` — indexed closure lookups
- `cpt-cf-resource-group-nfr-transactional-consistency` — SERIALIZABLE transactions
- `cpt-cf-resource-group-principle-strict-forest` — single parent and cycle prevention
- `cpt-cf-resource-group-principle-query-profile-guardrail` — depth/width as service profile

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-instance-administrator` | Manages groups globally, seeds hierarchy at deployment, configures query profile |
| `cpt-cf-resource-group-actor-tenant-administrator` | Manages groups within tenant scope — creates sub-groups, moves subtrees, deletes groups |
| `cpt-cf-resource-group-actor-apps` | Programmatic group management via `ResourceGroupClient` SDK |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) — `cpt-cf-resource-group-feature-entity-hierarchy`
- **OpenAPI**: [openapi.yaml](../openapi.yaml) — `/groups`, `/groups/{group_id}`, `/groups/{group_id}/depth`
- **Migration**: [migration.sql](../migration.sql) — `resource_group`, `resource_group_closure` tables with indexes
- **Design Components**: `cpt-cf-resource-group-component-entity-service`, `cpt-cf-resource-group-component-hierarchy-service`
- **Design Sequences**: `cpt-cf-resource-group-seq-create-entity-with-parent`, `cpt-cf-resource-group-seq-move-subtree`
- **Design Constraints**: `cpt-cf-resource-group-constraint-profile-change-safety`
- **Dependencies**: `cpt-cf-resource-group-feature-type-management` (entities reference types for parent-child compatibility)

## 2. Actor Flows (CDSL)

### Create Group Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-group-create`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Group created with validated type, optional parent, closure rows inserted
- Root group created (no parent) when type permits root placement (empty string `""` in parents array)

**Error Scenarios**:
- Type not found — `NotFound`
- Parent group not found — `NotFound`
- Parent type incompatible — `InvalidParentType`
- Depth or width limit exceeded — `LimitViolation`
- Serialization conflict — bounded retry, deterministic error if retries exhausted

**Steps**:
1. [ ] - `p1` - Actor sends API: POST /api/resource-group/v1/groups ({ group_type, name, parent_id, tenant_id, external_id }) - `inst-grp-create-1`
2. [ ] - `p1` - DB: BEGIN transaction (SERIALIZABLE) - `inst-grp-create-2`
3. [ ] - `p1` - DB: SELECT code, parents FROM resource_group_type WHERE code = :group_type — load type definition - `inst-grp-create-3`
4. [ ] - `p1` - **IF** type not found - `inst-grp-create-4`
   1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-create-4a`
   2. [ ] - `p1` - **RETURN** `NotFound` error — type does not exist - `inst-grp-create-4b`
5. [ ] - `p1` - **IF** parent_id is provided - `inst-grp-create-5`
   1. [ ] - `p1` - DB: SELECT id, group_type FROM resource_group WHERE id = :parent_id — load parent - `inst-grp-create-5a`
   2. [ ] - `p1` - **IF** parent not found - `inst-grp-create-5b`
      1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-create-5b1`
      2. [ ] - `p1` - **RETURN** `NotFound` error — parent group does not exist - `inst-grp-create-5b2`
   3. [ ] - `p1` - Invoke parent type compatibility validation (`cpt-cf-resource-group-algo-parent-type-compat`) — verify parent's group_type is in child type's parents array - `inst-grp-create-5c`
   4. [ ] - `p1` - **IF** incompatible - `inst-grp-create-5d`
      1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-create-5d1`
      2. [ ] - `p1` - **RETURN** `InvalidParentType` error with type codes - `inst-grp-create-5d2`
6. [ ] - `p1` - **ELSE** (no parent_id — root group) - `inst-grp-create-6`
   1. [ ] - `p1` - Verify type permits root placement (empty string `""` in type's parents array) - `inst-grp-create-6a`
   2. [ ] - `p1` - **IF** type does not permit root placement - `inst-grp-create-6b`
      1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-create-6b1`
      2. [ ] - `p1` - **RETURN** `InvalidParentType` error — type requires a parent - `inst-grp-create-6b2`
7. [ ] - `p1` - Invoke query profile enforcement (`cpt-cf-resource-group-algo-profile-enforcement`) — check depth/width limits - `inst-grp-create-7`
8. [ ] - `p1` - **IF** profile limits exceeded - `inst-grp-create-8`
   1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-create-8a`
   2. [ ] - `p1` - **RETURN** `LimitViolation` error with limit name and values - `inst-grp-create-8b`
9. [ ] - `p1` - DB: INSERT INTO resource_group (id, parent_id, group_type, name, tenant_id, external_id, created) — new UUID for id - `inst-grp-create-9`
10. [ ] - `p1` - DB: INSERT INTO resource_group_closure (ancestor_id, descendant_id, depth) VALUES (:new_id, :new_id, 0) — self-row - `inst-grp-create-10`
11. [ ] - `p1` - **IF** parent_id is provided - `inst-grp-create-11`
    1. [ ] - `p1` - DB: INSERT INTO resource_group_closure SELECT ancestor_id, :new_id, depth + 1 FROM resource_group_closure WHERE descendant_id = :parent_id — ancestor-descendant rows for all ancestors of parent - `inst-grp-create-11a`
12. [ ] - `p1` - DB: COMMIT - `inst-grp-create-12`
13. [ ] - `p1` - **IF** serialization conflict - `inst-grp-create-13`
    1. [ ] - `p1` - DB: ROLLBACK and retry (bounded retry policy) - `inst-grp-create-13a`
    2. [ ] - `p1` - **IF** retries exhausted — **RETURN** `ServiceUnavailable` error - `inst-grp-create-13b`
14. [ ] - `p1` - **RETURN** created `ResourceGroup` { group_id, parent_id, group_type, name, tenant_id, external_id } - `inst-grp-create-14`

### Get Group Flow

- [ ] `p2` - **ID**: `cpt-cf-resource-group-flow-group-get`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Group returned by ID

**Error Scenarios**:
- Group not found

**Steps**:
1. [ ] - `p2` - Actor sends API: GET /api/resource-group/v1/groups/{group_id} - `inst-grp-get-1`
2. [ ] - `p2` - DB: SELECT id, parent_id, group_type, name, tenant_id, external_id FROM resource_group WHERE id = :group_id - `inst-grp-get-2`
3. [ ] - `p2` - **IF** row found - `inst-grp-get-3`
   1. [ ] - `p2` - **RETURN** `ResourceGroup` - `inst-grp-get-3a`
4. [ ] - `p2` - **ELSE** - `inst-grp-get-4`
   1. [ ] - `p2` - **RETURN** `NotFound` error - `inst-grp-get-4a`

### List Groups Flow

- [ ] `p2` - **ID**: `cpt-cf-resource-group-flow-group-list`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Paginated list returned sorted by `group_id` ASC

**Error Scenarios**:
- Invalid OData filter — `Validation` error

**Steps**:
1. [ ] - `p2` - Actor sends API: GET /api/resource-group/v1/groups?$filter={expr}&$top={n}&$skip={m} - `inst-grp-list-1`
2. [ ] - `p2` - Parse OData: `$filter` on group_type (eq, ne, in), parent_id (eq, ne, in), group_id (eq, ne, in), name (eq, ne, in, contains, startswith, endswith), external_id (eq, ne, in, contains, startswith, endswith); `$top` (1..300, default 50); `$skip` (default 0) - `inst-grp-list-2`
3. [ ] - `p2` - **IF** OData parse fails - `inst-grp-list-3`
   1. [ ] - `p2` - **RETURN** `Validation` error - `inst-grp-list-3a`
4. [ ] - `p2` - DB: SELECT ... FROM resource_group WHERE {filter} ORDER BY id ASC LIMIT $top OFFSET $skip - `inst-grp-list-4`
5. [ ] - `p2` - **RETURN** `Page<ResourceGroup>` { items, page_info } - `inst-grp-list-5`

### Update Group Flow (Including Parent Move)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-group-update`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Group fields updated (name, external_id, group_type)
- If parent_id changed, subtree move executes with closure recalculation

**Error Scenarios**:
- Group not found
- New parent not found
- New parent type incompatible — `InvalidParentType`
- Move would create cycle — `CycleDetected`
- Move violates depth/width limits — `LimitViolation`
- Serialization conflict — bounded retry

**Steps**:
1. [ ] - `p1` - Actor sends API: PUT /api/resource-group/v1/groups/{group_id} ({ group_type, name, parent_id, external_id }) - `inst-grp-update-1`
2. [ ] - `p1` - DB: BEGIN transaction (SERIALIZABLE) - `inst-grp-update-2`
3. [ ] - `p1` - DB: SELECT * FROM resource_group WHERE id = :group_id — load current group - `inst-grp-update-3`
4. [ ] - `p1` - **IF** group not found - `inst-grp-update-4`
   1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-update-4a`
   2. [ ] - `p1` - **RETURN** `NotFound` error - `inst-grp-update-4b`
5. [ ] - `p1` - **IF** group_type changed — invoke parent type compatibility for current parent against new type - `inst-grp-update-5`
6. [ ] - `p1` - **IF** parent_id changed (move operation) - `inst-grp-update-6`
   1. [ ] - `p1` - **IF** new parent_id is not null - `inst-grp-update-6a`
      1. [ ] - `p1` - DB: load new parent group - `inst-grp-update-6a1`
      2. [ ] - `p1` - **IF** new parent not found — DB: ROLLBACK, **RETURN** `NotFound` - `inst-grp-update-6a2`
      3. [ ] - `p1` - Invoke parent type compatibility validation (`cpt-cf-resource-group-algo-parent-type-compat`) - `inst-grp-update-6a3`
      4. [ ] - `p1` - **IF** incompatible — DB: ROLLBACK, **RETURN** `InvalidParentType` - `inst-grp-update-6a4`
   2. [ ] - `p1` - Invoke cycle detection (`cpt-cf-resource-group-algo-cycle-detection`) — verify new parent is not in the subtree of the moving node - `inst-grp-update-6b`
   3. [ ] - `p1` - **IF** cycle detected — DB: ROLLBACK, **RETURN** `CycleDetected` - `inst-grp-update-6c`
   4. [ ] - `p1` - Invoke query profile enforcement (`cpt-cf-resource-group-algo-profile-enforcement`) for new position - `inst-grp-update-6d`
   5. [ ] - `p1` - **IF** limits exceeded — DB: ROLLBACK, **RETURN** `LimitViolation` - `inst-grp-update-6e`
   6. [ ] - `p1` - Invoke closure recalculation (`cpt-cf-resource-group-algo-closure-recalc`) — delete old paths, insert rebuilt paths - `inst-grp-update-6f`
7. [ ] - `p1` - DB: UPDATE resource_group SET group_type, name, parent_id, external_id, modified = NOW() WHERE id = :group_id - `inst-grp-update-7`
8. [ ] - `p1` - DB: COMMIT - `inst-grp-update-8`
9. [ ] - `p1` - **IF** serialization conflict — ROLLBACK and retry (bounded retry) - `inst-grp-update-9`
10. [ ] - `p1` - **RETURN** updated `ResourceGroup` - `inst-grp-update-10`

### Delete Group Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-group-delete`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Group deleted (no children, no memberships) — 204
- Force delete — cascade subtree and memberships, then delete — 204

**Error Scenarios**:
- Group not found — `NotFound`
- Active references (children or memberships) and force=false — `ConflictActiveReferences`

**Steps**:
1. [ ] - `p1` - Actor sends API: DELETE /api/resource-group/v1/groups/{group_id}?force={bool} - `inst-grp-delete-1`
2. [ ] - `p1` - DB: BEGIN transaction (SERIALIZABLE) - `inst-grp-delete-2`
3. [ ] - `p1` - DB: SELECT id FROM resource_group WHERE id = :group_id — verify existence - `inst-grp-delete-3`
4. [ ] - `p1` - **IF** group not found - `inst-grp-delete-4`
   1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-delete-4a`
   2. [ ] - `p1` - **RETURN** `NotFound` error - `inst-grp-delete-4b`
5. [ ] - `p1` - **IF** force = false - `inst-grp-delete-5`
   1. [ ] - `p1` - DB: SELECT COUNT(*) FROM resource_group WHERE parent_id = :group_id — count children - `inst-grp-delete-5a`
   2. [ ] - `p1` - DB: SELECT COUNT(*) FROM resource_group_membership WHERE group_id = :group_id — count memberships - `inst-grp-delete-5b`
   3. [ ] - `p1` - **IF** children > 0 OR memberships > 0 - `inst-grp-delete-5c`
      1. [ ] - `p1` - DB: ROLLBACK - `inst-grp-delete-5c1`
      2. [ ] - `p1` - **RETURN** `ConflictActiveReferences` error with reference counts - `inst-grp-delete-5c2`
   4. [ ] - `p1` - DB: DELETE FROM resource_group_closure WHERE ancestor_id = :group_id OR descendant_id = :group_id — remove closure rows - `inst-grp-delete-5d`
   5. [ ] - `p1` - DB: DELETE FROM resource_group WHERE id = :group_id - `inst-grp-delete-5e`
6. [ ] - `p2` - **ELSE** (force = true) - `inst-grp-delete-6`
   1. [ ] - `p2` - Invoke force delete cascade (`cpt-cf-resource-group-algo-force-delete-cascade`) — delete subtree and memberships recursively - `inst-grp-delete-6a`
7. [ ] - `p1` - DB: COMMIT - `inst-grp-delete-7`
8. [ ] - `p1` - **IF** serialization conflict — ROLLBACK and retry - `inst-grp-delete-8`
9. [ ] - `p1` - **RETURN** success (204 No Content) - `inst-grp-delete-9`

### List Group Depth Flow (Hierarchy Traversal)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-group-depth`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Groups returned with relative depth from reference group
- Ancestors (negative depth), self (depth=0), descendants (positive depth) in one query

**Error Scenarios**:
- Reference group not found — `NotFound`
- Invalid OData filter — `Validation`

**Steps**:
1. [ ] - `p1` - Actor sends API: GET /api/resource-group/v1/groups/{group_id}/depth?$filter={expr}&$top={n}&$skip={m} - `inst-grp-depth-1`
2. [ ] - `p1` - DB: SELECT id FROM resource_group WHERE id = :group_id — verify reference group exists - `inst-grp-depth-2`
3. [ ] - `p1` - **IF** reference group not found - `inst-grp-depth-3`
   1. [ ] - `p1` - **RETURN** `NotFound` error - `inst-grp-depth-3a`
4. [ ] - `p1` - Parse OData: `$filter` on depth (eq, ne, gt, ge, lt, le), group_type (eq, ne, in); `$top` (1..300, default 50); `$skip` (default 0) - `inst-grp-depth-4`
5. [ ] - `p1` - **IF** OData parse fails - `inst-grp-depth-5`
   1. [ ] - `p1` - **RETURN** `Validation` error - `inst-grp-depth-5a`
6. [ ] - `p1` - Compute descendant query: DB: SELECT rg.*, c.depth FROM resource_group rg JOIN resource_group_closure c ON rg.id = c.descendant_id WHERE c.ancestor_id = :group_id AND depth filter (positive range) - `inst-grp-depth-6`
7. [ ] - `p1` - Compute ancestor query: DB: SELECT rg.*, -c.depth as depth FROM resource_group rg JOIN resource_group_closure c ON rg.id = c.ancestor_id WHERE c.descendant_id = :group_id AND depth filter (negative range, negate stored depth) - `inst-grp-depth-7`
8. [ ] - `p1` - Merge results, apply group_type filter, sort by depth ASC then group_id ASC - `inst-grp-depth-8`
9. [ ] - `p1` - Apply pagination ($top, $skip) to merged result - `inst-grp-depth-9`
10. [ ] - `p1` - **RETURN** `Page<ResourceGroupWithDepth>` { items, page_info } - `inst-grp-depth-10`

### Seed Groups Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-group-seed`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- All seed group definitions created/updated deterministically with closure rows
- Parent-child relationships validated against type rules

**Error Scenarios**:
- Seed data references non-existent type — seed aborts
- Seed data creates cycle — seed aborts

**Steps**:
1. [ ] - `p1` - Instance Administrator triggers seed operation (pre-deployment, system SecurityContext) - `inst-grp-seed-1`
2. [ ] - `p1` - **FOR EACH** group definition in seed data (ordered by dependency — parents before children) - `inst-grp-seed-2`
   1. [ ] - `p1` - Validate group_type exists via DB lookup - `inst-grp-seed-2a`
   2. [ ] - `p1` - **IF** type not found — **RETURN** `NotFound` error, seed aborts - `inst-grp-seed-2b`
   3. [ ] - `p1` - **IF** parent_id specified — validate parent exists and type compatibility - `inst-grp-seed-2c`
   4. [ ] - `p1` - DB: UPSERT resource_group (id, parent_id, group_type, name, tenant_id, external_id) — insert or update on conflict - `inst-grp-seed-2d`
   5. [ ] - `p1` - Maintain closure table rows (self-row + ancestor paths) - `inst-grp-seed-2e`
3. [ ] - `p1` - **RETURN** seed complete — hierarchy provisioned - `inst-grp-seed-3`

## 3. Processes / Business Logic (CDSL)

### Parent Type Compatibility Validation

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-parent-type-compat`

**Input**: Child type code, parent group's type code (or null for root)

**Output**: Compatible (allow) or incompatible (reject with type codes)

**Steps**:
1. [ ] - `p1` - DB: SELECT parents FROM resource_group_type WHERE code = :child_type_code — load child type definition - `inst-compat-1`
2. [ ] - `p1` - **IF** parent is null (root placement) - `inst-compat-2`
   1. [ ] - `p1` - **IF** child type's parents array contains empty string `""` - `inst-compat-2a`
      1. [ ] - `p1` - **RETURN** compatible — type permits root placement - `inst-compat-2a1`
   2. [ ] - `p1` - **ELSE** - `inst-compat-2b`
      1. [ ] - `p1` - **RETURN** incompatible — type requires a parent - `inst-compat-2b1`
3. [ ] - `p1` - **IF** parent's group_type is in child type's parents array - `inst-compat-3`
   1. [ ] - `p1` - **RETURN** compatible - `inst-compat-3a`
4. [ ] - `p1` - **ELSE** - `inst-compat-4`
   1. [ ] - `p1` - **RETURN** incompatible — parent type not in allowed parents list - `inst-compat-4a`

### Cycle Detection

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-cycle-detection`

**Input**: Moving node ID, proposed new parent ID

**Output**: No cycle (allow) or cycle detected (reject)

**Steps**:
1. [ ] - `p1` - DB: SELECT ancestor_id FROM resource_group_closure WHERE descendant_id = :new_parent_id AND ancestor_id = :moving_node_id — check if moving node is an ancestor of proposed new parent - `inst-cycle-1`
2. [ ] - `p1` - **IF** row found (moving node is ancestor of new parent) - `inst-cycle-2`
   1. [ ] - `p1` - **RETURN** cycle detected — moving a node under its own descendant creates a cycle - `inst-cycle-2a`
3. [ ] - `p1` - **ELSE** - `inst-cycle-3`
   1. [ ] - `p1` - **RETURN** no cycle — move is safe - `inst-cycle-3a`

### Closure Table Recalculation (Subtree Move)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-closure-recalc`

**Input**: Moving node ID, old parent ID, new parent ID

**Output**: Closure table updated — old ancestor paths removed, new ancestor paths inserted

**Steps**:
1. [ ] - `p1` - Identify subtree: DB: SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = :moving_node_id — all nodes in subtree (including self) - `inst-closure-1`
2. [ ] - `p1` - Delete old external paths: DB: DELETE FROM resource_group_closure WHERE descendant_id IN (subtree) AND ancestor_id NOT IN (subtree) — remove paths from old ancestors to all subtree nodes - `inst-closure-2`
3. [ ] - `p1` - Insert new external paths: DB: INSERT INTO resource_group_closure SELECT a.ancestor_id, d.descendant_id, a.depth + d.depth + 1 FROM resource_group_closure a CROSS JOIN resource_group_closure d WHERE a.descendant_id = :new_parent_id AND d.ancestor_id = :moving_node_id — create paths from new ancestors to all subtree nodes - `inst-closure-3`
4. [ ] - `p1` - **RETURN** closure recalculation complete - `inst-closure-4`

### Query Profile Enforcement

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-profile-enforcement`

**Input**: Proposed write operation context (new node position, parent depth)

**Output**: Allow or reject with limit details

**Steps**:
1. [ ] - `p1` - Load query profile config: max_depth, max_width - `inst-profile-1`
2. [ ] - `p1` - **IF** max_depth is enabled (not null/disabled) - `inst-profile-2`
   1. [ ] - `p1` - Calculate resulting depth of new node from root - `inst-profile-2a`
   2. [ ] - `p1` - **IF** resulting depth > max_depth - `inst-profile-2b`
      1. [ ] - `p1` - **RETURN** `LimitViolation` — depth limit exceeded (current: {depth}, max: {max_depth}) - `inst-profile-2b1`
3. [ ] - `p1` - **IF** max_width is enabled (not null/disabled) - `inst-profile-3`
   1. [ ] - `p1` - DB: count direct children of target parent - `inst-profile-3a`
   2. [ ] - `p1` - **IF** child count + 1 > max_width - `inst-profile-3b`
      1. [ ] - `p1` - **RETURN** `LimitViolation` — width limit exceeded (current: {count}, max: {max_width}) - `inst-profile-3b1`
4. [ ] - `p1` - **RETURN** allow — within profile limits - `inst-profile-4`

### Force Delete Cascade

- [ ] `p2` - **ID**: `cpt-cf-resource-group-algo-force-delete-cascade`

**Input**: Group ID to force-delete

**Output**: Subtree and all associated memberships deleted

**Steps**:
1. [ ] - `p2` - DB: SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = :group_id ORDER BY depth DESC — all subtree nodes, deepest first - `inst-force-1`
2. [ ] - `p2` - **FOR EACH** descendant in subtree (deepest first) - `inst-force-2`
   1. [ ] - `p2` - DB: DELETE FROM resource_group_membership WHERE group_id = :descendant — remove all memberships for this node - `inst-force-2a`
   2. [ ] - `p2` - DB: DELETE FROM resource_group_closure WHERE ancestor_id = :descendant OR descendant_id = :descendant — remove all closure rows for this node - `inst-force-2b`
   3. [ ] - `p2` - DB: DELETE FROM resource_group WHERE id = :descendant — remove the group itself - `inst-force-2c`
3. [ ] - `p2` - **RETURN** cascade complete — subtree and memberships removed - `inst-force-3`

## 4. States (CDSL)

Not applicable. Resource groups do not have lifecycle states in this feature. Groups exist in the hierarchy or they do not — creation and deletion are the boundary events. Future features may introduce soft-delete states if needed.

## 5. Definitions of Done

### Entity Create with Forest Invariants

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-create`

The system **MUST** create a resource group via `POST /api/resource-group/v1/groups` accepting `{ group_type, name, parent_id, tenant_id, external_id }`. The system **MUST** validate that the type exists, that the parent (if provided) exists, and that the parent type is compatible per the type's `parents` array. For root groups (no parent_id), the type **MUST** permit root placement (empty string `""` in parents). The system **MUST** enforce depth/width limits per the query profile. The system **MUST** insert the entity row and all closure rows (self-row + ancestor paths) within a single SERIALIZABLE transaction. Serialization conflicts **MUST** trigger bounded retry with deterministic error on exhaustion.

**Implements**:
- `cpt-cf-resource-group-flow-group-create`
- `cpt-cf-resource-group-algo-parent-type-compat`
- `cpt-cf-resource-group-algo-profile-enforcement`

**Touches**:
- API: `POST /api/resource-group/v1/groups`
- DB: `resource_group`, `resource_group_closure`, `resource_group_type` (read)
- Entities: `ResourceGroup`, `ResourceGroupClosure`

### Entity Read Operations

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-read`

The system **MUST** retrieve a single group by ID via `GET /api/resource-group/v1/groups/{group_id}` returning `ResourceGroup` or `NotFound`. The system **MUST** list groups via `GET /api/resource-group/v1/groups` with OData `$filter` on group_type (eq, ne, in), parent_id (eq, ne, in), group_id (eq, ne, in), name (eq, ne, in, contains, startswith, endswith), external_id (eq, ne, in, contains, startswith, endswith). Results **MUST** be sorted by `group_id` ASC. Group responses **MUST NOT** include `created`/`modified` timestamps per DESIGN API projection rules.

**Implements**:
- `cpt-cf-resource-group-flow-group-get`
- `cpt-cf-resource-group-flow-group-list`

**Touches**:
- API: `GET /api/resource-group/v1/groups`, `GET /api/resource-group/v1/groups/{group_id}`
- DB: `resource_group`
- Entities: `ResourceGroup`

### Entity Update and Subtree Move

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-update`

The system **MUST** update a group via `PUT /api/resource-group/v1/groups/{group_id}` accepting `{ group_type, name, parent_id, external_id }`. When `parent_id` changes (move operation), the system **MUST**: validate the new parent exists; validate parent type compatibility; detect cycles via closure table lookup; enforce query profile limits for the new position; recalculate closure table by deleting old ancestor paths and inserting new ancestor paths — all within a single SERIALIZABLE transaction. When `group_type` changes, the system **MUST** re-validate parent type compatibility against the current parent. The `modified` timestamp **MUST** be set.

**Implements**:
- `cpt-cf-resource-group-flow-group-update`
- `cpt-cf-resource-group-algo-parent-type-compat`
- `cpt-cf-resource-group-algo-cycle-detection`
- `cpt-cf-resource-group-algo-closure-recalc`
- `cpt-cf-resource-group-algo-profile-enforcement`

**Touches**:
- API: `PUT /api/resource-group/v1/groups/{group_id}`
- DB: `resource_group`, `resource_group_closure`
- Entities: `ResourceGroup`, `ResourceGroupClosure`

### Entity Delete with Guard and Force Cascade

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-entity-delete`

The system **MUST** delete a group via `DELETE /api/resource-group/v1/groups/{group_id}?force={bool}`. With `force=false` (default): the system **MUST** check for children (via `parent_id` FK) and memberships (via `group_id` FK) and reject with `ConflictActiveReferences` if any exist; otherwise delete the group and its closure rows. With `force=true`: the system **MUST** cascade-delete the entire subtree (deepest descendants first) including all associated memberships and closure rows, within a single SERIALIZABLE transaction.

**Implements**:
- `cpt-cf-resource-group-flow-group-delete`
- `cpt-cf-resource-group-algo-force-delete-cascade`

**Touches**:
- API: `DELETE /api/resource-group/v1/groups/{group_id}`
- DB: `resource_group`, `resource_group_closure`, `resource_group_membership` (force delete)
- Entities: `ResourceGroup`, `ResourceGroupClosure`, `ResourceGroupMembership`

### Depth-Based Hierarchy Traversal

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-depth-traversal`

The system **MUST** provide hierarchy traversal via `GET /api/resource-group/v1/groups/{group_id}/depth` returning `Page<ResourceGroupWithDepth>` with computed relative `depth` field: `0` = reference group, positive = descendants, negative = ancestors. OData `$filter` **MUST** support `depth` (eq, ne, gt, ge, lt, le) and `group_type` (eq, ne, in). Negative depth **MUST** be computed by reversing the closure table lookup direction and negating the stored depth. Results **MUST** be sorted by `depth` ASC, `group_id` ASC. The reference group **MUST** be included when the depth range covers `0`.

**Implements**:
- `cpt-cf-resource-group-flow-group-depth`

**Touches**:
- API: `GET /api/resource-group/v1/groups/{group_id}/depth`
- DB: `resource_group`, `resource_group_closure`
- Entities: `ResourceGroupWithDepth`

### Query Profile Enforcement

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-query-profile`

The system **MUST** enforce `max_depth` and `max_width` limits from the query profile configuration on all hierarchy-mutating writes (create, move). When a write would exceed a limit, the system **MUST** reject with `LimitViolation`. Tightened profile limits **MUST NOT** rewrite existing closure rows per `cpt-cf-resource-group-constraint-profile-change-safety` — existing data that violates new limits is preserved for reads, but new writes that would worsen the violation are rejected. Profile limits may be disabled (null), in which case no enforcement applies.

**Implements**:
- `cpt-cf-resource-group-algo-profile-enforcement`

**Touches**:
- DB: `resource_group_closure` (depth calculation), `resource_group` (width calculation)

### Seed Groups Path

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-group-seed`

The system **MUST** provide a deterministic seed path that creates/updates group hierarchy definitions at pre-deployment time. Seed operations **MUST** run with system `SecurityContext` (bypassing AuthZ). Seed **MUST** process groups in dependency order (parents before children), validate type existence and parent-child compatibility, create closure table rows, and abort on the first validation error. The seed path **MUST** be idempotent.

**Implements**:
- `cpt-cf-resource-group-flow-group-seed`

**Touches**:
- DB: `resource_group`, `resource_group_closure`

## 6. Acceptance Criteria

- [ ] `POST /groups` creates group and returns `201` with `ResourceGroup`
- [ ] Root group (null parent_id) is accepted when type permits root placement
- [ ] Root group is rejected with `InvalidParentType` when type does not permit root placement
- [ ] Parent type compatibility is validated — incompatible parent returns `InvalidParentType`
- [ ] Non-existent type returns `NotFound`
- [ ] Non-existent parent returns `NotFound`
- [ ] Closure self-row (depth=0) is inserted on create
- [ ] Ancestor-descendant closure rows are inserted for all ancestors of the parent
- [ ] All create validations and inserts execute within a single SERIALIZABLE transaction
- [ ] `GET /groups/{group_id}` returns group or `404`
- [ ] `GET /groups` returns paginated list sorted by `group_id` ASC
- [ ] OData `$filter` works on group_type, parent_id, group_id, name, external_id with specified operators
- [ ] `PUT /groups/{group_id}` updates fields and returns updated group
- [ ] Parent move (changed parent_id) triggers closure recalculation within SERIALIZABLE transaction
- [ ] Move to own descendant is rejected with `CycleDetected`
- [ ] Move validates type compatibility with new parent
- [ ] Move validates depth/width profile limits for new position
- [ ] `DELETE /groups/{group_id}` returns `204` when no active references and force=false
- [ ] Delete is rejected with `ConflictActiveReferences` when children or memberships exist and force=false
- [ ] Delete with `force=true` cascades subtree and memberships (deepest first)
- [ ] `GET /groups/{group_id}/depth` returns groups with relative depth (negative=ancestors, 0=self, positive=descendants)
- [ ] Depth endpoint applies OData `$filter` on depth (eq, ne, gt, ge, lt, le) and group_type (eq, ne, in)
- [ ] Depth results sorted by depth ASC, group_id ASC
- [ ] Reference group included when depth range covers 0
- [ ] Non-existent reference group returns `404`
- [ ] Query profile `max_depth` rejects creates/moves exceeding depth limit with `LimitViolation`
- [ ] Query profile `max_width` rejects creates/moves exceeding width limit with `LimitViolation`
- [ ] Tightened profile limits do not rewrite existing closure rows
- [ ] Tightened profile limits reject new writes that worsen violations
- [ ] Disabled profile limits (null) allow unlimited depth/width
- [ ] Serialization conflicts trigger bounded retry with deterministic error on exhaustion
- [ ] Seed groups path creates hierarchy deterministically and is idempotent
- [ ] Seed aborts on first type or compatibility error
- [ ] Group responses do not include `created`/`modified` timestamps

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — groups have no explicit lifecycle states in the current design.
- **Usability (UX)**: Not applicable — backend API only, no frontend.
- **Compliance (COMPL)**: Not applicable — compliance controls are platform-level; groups do not contain regulated data directly.
- **External Integrations**: Not applicable — entity/hierarchy management is internal to RG module. Integration read exposure is Feature 5.
