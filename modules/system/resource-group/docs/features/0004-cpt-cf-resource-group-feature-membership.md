# Feature: Membership Management

- [ ] `p2` - **ID**: `cpt-cf-resource-group-featstatus-membership`
- [ ] `p2` - `cpt-cf-resource-group-feature-membership`

## 1. Feature Context

### 1.1 Overview

Implement membership CRUD — add, remove, and list resource-to-group links with tenant-scoped ownership-graph validation, duplicate prevention via unique constraint, deterministic seed path, and indexed reverse lookups by resource.

### 1.2 Purpose

Memberships bind resources (users, courses, assets) to groups, forming the many-to-many relationship layer on top of the group hierarchy. In ownership-graph profile, membership writes enforce tenant scope compatibility. Without memberships, the integration read contract (Feature 5) cannot provide resource-level context to AuthZ plugins.

Addresses:
- `cpt-cf-resource-group-fr-manage-membership` — add/remove lifecycle
- `cpt-cf-resource-group-fr-query-membership-relations` — indexed lookups by group and by resource
- `cpt-cf-resource-group-fr-seed-memberships` — deterministic pre-deployment seeding
- `cpt-cf-resource-group-fr-tenant-scope-ownership-graph` — tenant scope validation on writes
- `cpt-cf-resource-group-nfr-membership-query-latency` — direct lookup by group/resource keys
- `cpt-cf-resource-group-principle-tenant-scope-ownership-graph` — tenant-scoped ownership-graph semantics

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-tenant-administrator` | Manages memberships within tenant scope via REST API |
| `cpt-cf-resource-group-actor-instance-administrator` | Seeds memberships at deployment |
| `cpt-cf-resource-group-actor-apps` | Programmatic membership management via `ResourceGroupClient` SDK |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) — `cpt-cf-resource-group-feature-membership`
- **OpenAPI**: [openapi.yaml](../openapi.yaml) — `/memberships`, `/memberships/{group_id}/{resource_type}/{resource_id}`
- **Migration**: [migration.sql](../migration.sql) — `resource_group_membership` table, `uq_resource_group_membership_unique`, `idx_rgm_resource_type_id`
- **Design Components**: `cpt-cf-resource-group-component-membership-service`
- **Dependencies**: `cpt-cf-resource-group-feature-entity-hierarchy` (memberships reference groups that must exist)

## 2. Actor Flows (CDSL)

### Add Membership Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-membership-add`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Membership created linking resource to group — returns `ResourceGroupMembership`

**Error Scenarios**:
- Group not found — `NotFound`
- Duplicate membership (same group_id, resource_type, resource_id) — `ConflictActiveReferences`
- Tenant scope incompatible (ownership-graph profile) — `TenantIncompatibility`

**Steps**:
1. [ ] - `p1` - Actor sends API: POST /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id} - `inst-mbr-add-1`
2. [ ] - `p1` - DB: SELECT id, tenant_id FROM resource_group WHERE id = :group_id — verify group exists and load tenant_id - `inst-mbr-add-2`
3. [ ] - `p1` - **IF** group not found - `inst-mbr-add-3`
   1. [ ] - `p1` - **RETURN** `NotFound` error — group does not exist - `inst-mbr-add-3a`
4. [ ] - `p1` - Invoke tenant scope validation (`cpt-cf-resource-group-algo-membership-tenant-scope`) — verify caller's effective scope is compatible with group's tenant_id - `inst-mbr-add-4`
5. [ ] - `p1` - **IF** tenant scope incompatible - `inst-mbr-add-5`
   1. [ ] - `p1` - **RETURN** `TenantIncompatibility` error with tenant context - `inst-mbr-add-5a`
6. [ ] - `p1` - DB: INSERT INTO resource_group_membership (group_id, resource_type, resource_id, created) - `inst-mbr-add-6`
7. [ ] - `p1` - **IF** unique constraint violation on (group_id, resource_type, resource_id) - `inst-mbr-add-7`
   1. [ ] - `p1` - **RETURN** `ConflictActiveReferences` error — membership already exists - `inst-mbr-add-7a`
8. [ ] - `p1` - **RETURN** created `ResourceGroupMembership` { group_id, resource_type, resource_id } (201 Created) - `inst-mbr-add-8`

### Remove Membership Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-membership-remove`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Membership removed — 204 No Content

**Error Scenarios**:
- Membership not found — `NotFound`
- Tenant scope incompatible — `TenantIncompatibility`

**Steps**:
1. [ ] - `p1` - Actor sends API: DELETE /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id} - `inst-mbr-remove-1`
2. [ ] - `p1` - DB: SELECT group_id, resource_type, resource_id FROM resource_group_membership WHERE group_id = :group_id AND resource_type = :resource_type AND resource_id = :resource_id — verify existence - `inst-mbr-remove-2`
3. [ ] - `p1` - **IF** membership not found - `inst-mbr-remove-3`
   1. [ ] - `p1` - **RETURN** `NotFound` error - `inst-mbr-remove-3a`
4. [ ] - `p1` - DB: SELECT tenant_id FROM resource_group WHERE id = :group_id — load group tenant for scope check - `inst-mbr-remove-4`
5. [ ] - `p1` - Invoke tenant scope validation (`cpt-cf-resource-group-algo-membership-tenant-scope`) - `inst-mbr-remove-5`
6. [ ] - `p1` - **IF** tenant scope incompatible - `inst-mbr-remove-6`
   1. [ ] - `p1` - **RETURN** `TenantIncompatibility` error - `inst-mbr-remove-6a`
7. [ ] - `p1` - DB: DELETE FROM resource_group_membership WHERE group_id = :group_id AND resource_type = :resource_type AND resource_id = :resource_id - `inst-mbr-remove-7`
8. [ ] - `p1` - **RETURN** success (204 No Content) - `inst-mbr-remove-8`

### List Memberships Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-membership-list`

**Actor**: `cpt-cf-resource-group-actor-tenant-administrator`

**Success Scenarios**:
- Paginated list of memberships returned sorted by group_id ASC, resource_type ASC, resource_id ASC
- OData `$filter` applied on group_id, resource_type, resource_id

**Error Scenarios**:
- Invalid OData filter — `Validation` error

**Steps**:
1. [ ] - `p1` - Actor sends API: GET /api/resource-group/v1/memberships?$filter={expr}&$top={n}&$skip={m} - `inst-mbr-list-1`
2. [ ] - `p1` - Parse OData: `$filter` on resource_id (eq, ne, in, contains, startswith, endswith), resource_type (eq, ne, in), group_id (eq, ne, in); `$top` (1..300, default 50); `$skip` (default 0) - `inst-mbr-list-2`
3. [ ] - `p1` - **IF** OData parse fails - `inst-mbr-list-3`
   1. [ ] - `p1` - **RETURN** `Validation` error - `inst-mbr-list-3a`
4. [ ] - `p1` - DB: SELECT group_id, resource_type, resource_id FROM resource_group_membership WHERE {filter} ORDER BY group_id ASC, resource_type ASC, resource_id ASC LIMIT $top OFFSET $skip - `inst-mbr-list-4`
5. [ ] - `p1` - **RETURN** `Page<ResourceGroupMembership>` { items, page_info } - `inst-mbr-list-5`

### Seed Memberships Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-membership-seed`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- All seed membership definitions created deterministically
- Duplicate seed entries are idempotent (insert-or-ignore on unique constraint)

**Error Scenarios**:
- Seed data references non-existent group — seed aborts

**Steps**:
1. [ ] - `p1` - Instance Administrator triggers seed operation (pre-deployment, system SecurityContext) - `inst-mbr-seed-1`
2. [ ] - `p1` - **FOR EACH** membership definition in seed data - `inst-mbr-seed-2`
   1. [ ] - `p1` - DB: SELECT id, tenant_id FROM resource_group WHERE id = :group_id — verify group exists - `inst-mbr-seed-2a`
   2. [ ] - `p1` - **IF** group not found — **RETURN** `NotFound` error, seed aborts - `inst-mbr-seed-2b`
   3. [ ] - `p1` - DB: INSERT INTO resource_group_membership (group_id, resource_type, resource_id, created) ON CONFLICT (group_id, resource_type, resource_id) DO NOTHING — idempotent upsert - `inst-mbr-seed-2c`
3. [ ] - `p1` - **RETURN** seed complete — memberships provisioned - `inst-mbr-seed-3`

## 3. Processes / Business Logic (CDSL)

### Membership Tenant Scope Validation

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-membership-tenant-scope`

**Input**: Caller `SecurityContext` (with `subject_tenant_id` / effective scope), target group `tenant_id`

**Output**: Compatible (allow) or incompatible (reject with tenant context)

**Steps**:
1. [ ] - `p1` - **IF** ownership-graph profile is not active — **RETURN** compatible (no tenant scope enforcement in catalog profile) - `inst-tenant-1`
2. [ ] - `p1` - Extract caller effective scope from SecurityContext (`subject_tenant_id`) - `inst-tenant-2`
3. [ ] - `p1` - **IF** caller is platform-admin (privileged provisioning exception) - `inst-tenant-3`
   1. [ ] - `p1` - **RETURN** compatible — platform-admin provisioning bypasses tenant scope check for management operations - `inst-tenant-3a`
4. [ ] - `p1` - **IF** target group's tenant_id is within caller's effective tenant scope (same-tenant or allowed related-tenant per tenant hierarchy rules) - `inst-tenant-4`
   1. [ ] - `p1` - **RETURN** compatible - `inst-tenant-4a`
5. [ ] - `p1` - **ELSE** - `inst-tenant-5`
   1. [ ] - `p1` - **RETURN** incompatible — caller tenant scope does not cover target group tenant - `inst-tenant-5a`

## 4. States (CDSL)

Not applicable. Memberships are simple links with no lifecycle states — they exist (added) or they do not (removed). Creation and removal are atomic operations.

## 5. Definitions of Done

### Add Membership

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-membership-add`

The system **MUST** add a membership via `POST /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}` returning `ResourceGroupMembership` (201 Created). The system **MUST** verify the target group exists or return `NotFound`. In ownership-graph profile, the system **MUST** validate that the caller's effective tenant scope covers the target group's `tenant_id` (derived via JOIN), rejecting with `TenantIncompatibility` if incompatible. Platform-admin provisioning calls **MUST** bypass tenant scope validation. Duplicate membership (same composite key) **MUST** return `ConflictActiveReferences`.

**Implements**:
- `cpt-cf-resource-group-flow-membership-add`
- `cpt-cf-resource-group-algo-membership-tenant-scope`

**Touches**:
- API: `POST /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}`
- DB: `resource_group_membership`, `resource_group` (read for existence and tenant_id)
- Entities: `ResourceGroupMembership`

### Remove Membership

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-membership-remove`

The system **MUST** remove a membership via `DELETE /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}` returning 204 No Content. The system **MUST** verify the membership exists or return `NotFound`. In ownership-graph profile, the system **MUST** validate tenant scope compatibility before removal.

**Implements**:
- `cpt-cf-resource-group-flow-membership-remove`
- `cpt-cf-resource-group-algo-membership-tenant-scope`

**Touches**:
- API: `DELETE /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}`
- DB: `resource_group_membership`, `resource_group` (read for tenant_id)

### List Memberships with OData

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-membership-list`

The system **MUST** list memberships via `GET /api/resource-group/v1/memberships` with OData `$filter` on resource_id (eq, ne, in, contains, startswith, endswith), resource_type (eq, ne, in), group_id (eq, ne, in). Results **MUST** be sorted by `group_id` ASC, `resource_type` ASC, `resource_id` ASC. Membership responses **MUST NOT** include `tenant_id` — tenant scope is derived from group data the caller already has. Pagination via `$top` (1..300, default 50) and `$skip` (default 0).

**Implements**:
- `cpt-cf-resource-group-flow-membership-list`

**Touches**:
- API: `GET /api/resource-group/v1/memberships`
- DB: `resource_group_membership`
- Entities: `ResourceGroupMembership`

### Membership Active-Reference Guard

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-membership-active-ref-guard`

The system **MUST** protect group deletion when memberships exist. When `DELETE /groups/{group_id}` is called with `force=false` and memberships reference the group, the delete **MUST** be rejected with `ConflictActiveReferences`. This guard is implemented in the entity delete flow (Feature 3) but depends on membership data managed by this feature. The `resource_group_membership.group_id` FK with `ON DELETE RESTRICT` provides the database-level safety net.

**Implements**:
- `cpt-cf-resource-group-flow-membership-add` (creates references that activate the guard)

**Touches**:
- DB: `resource_group_membership` (FK constraint)

### Seed Memberships Path

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-membership-seed`

The system **MUST** provide a deterministic seed path that creates membership links at pre-deployment time. Seed operations **MUST** run with system `SecurityContext` (bypassing AuthZ and tenant scope validation). The seed path **MUST** verify that each target group exists, abort on the first `NotFound`, and handle duplicates idempotently (insert-or-ignore on unique constraint). The seed path **MUST** be idempotent.

**Implements**:
- `cpt-cf-resource-group-flow-membership-seed`

**Touches**:
- DB: `resource_group_membership`, `resource_group` (read for existence)

## 6. Acceptance Criteria

- [ ] `POST /memberships/{group_id}/{resource_type}/{resource_id}` creates membership and returns `201` with `ResourceGroupMembership`
- [ ] Add membership to non-existent group returns `NotFound`
- [ ] Duplicate membership (same composite key) returns `ConflictActiveReferences`
- [ ] In ownership-graph profile, add membership with incompatible tenant scope returns `TenantIncompatibility`
- [ ] Platform-admin add membership bypasses tenant scope validation
- [ ] `DELETE /memberships/{group_id}/{resource_type}/{resource_id}` removes membership and returns `204`
- [ ] Remove non-existent membership returns `NotFound`
- [ ] In ownership-graph profile, remove membership validates tenant scope
- [ ] `GET /memberships` returns paginated list sorted by group_id ASC, resource_type ASC, resource_id ASC
- [ ] OData `$filter` on resource_id works for eq, ne, in, contains, startswith, endswith
- [ ] OData `$filter` on resource_type works for eq, ne, in
- [ ] OData `$filter` on group_id works for eq, ne, in
- [ ] Membership responses do not include `tenant_id`
- [ ] Reverse lookup by resource_type + resource_id returns memberships across groups (uses `idx_rgm_resource_type_id` index)
- [ ] Group delete with `force=false` is rejected when memberships reference the group
- [ ] Group delete with `force=true` cascades membership removal
- [ ] Seed memberships path provisions links deterministically and is idempotent
- [ ] Seed aborts on first non-existent group reference
- [ ] Seed runs with system SecurityContext (bypasses AuthZ and tenant scope)
- [ ] Seed handles duplicate entries idempotently (insert-or-ignore)

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — memberships are stateless links; they exist or they do not.
- **Usability (UX)**: Not applicable — backend API only.
- **Compliance (COMPL)**: Not applicable — membership data is structural, not regulated.
- **Operations (OPS)**: Standard platform patterns. The `resource_group_membership` table may grow large at production scale (~455M rows per DESIGN section 4.1); partitioning strategy evaluation is a future concern, not in scope for this feature.
- **External Integrations**: Not applicable — membership management is internal. Integration read exposure is Feature 5.
