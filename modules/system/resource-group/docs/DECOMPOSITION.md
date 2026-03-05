# Decomposition: Resource Group (RG)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-status-overall`

## 1. Overview

The Resource Group (RG) DESIGN is decomposed into five features organized as a linear chain with a terminal fork.

**Decomposition strategy**: Features follow the natural dependency order of the RG architecture layers — persistence/SDK foundation first, then domain services (types → entities/hierarchy → memberships), and finally the integration read contract for AuthZ interop. Each feature maps to one or two DESIGN components with high cohesion and minimal cross-feature coupling.

| # | Feature | Priority | Components | Key Concern |
|---|---------|----------|------------|-------------|
| 1 | Domain Foundation | HIGH | module, persistence-adapter | SDK, module shell, DB, errors |
| 2 | Type Management | HIGH | type-service | Type lifecycle and validation |
| 3 | Entity & Hierarchy | HIGH | entity-service, hierarchy-service | Forest topology, closure table |
| 4 | Membership Management | MEDIUM | membership-service | Membership CRUD, tenant scope |
| 5 | Integration Read & AuthZ Interop | MEDIUM | integration-read-service | Read contracts, JWT/MTLS auth |

---

## 2. Entries

### 1. Domain Foundation - HIGH

- [ ] `p1` - **ID**: `cpt-cf-resource-group-feature-domain-foundation`

- **Purpose**: Establish the module skeleton — SDK crate (models, traits, errors), module lifecycle and ClientHub registration, persistence adapter with DB migration, and the unified error mapper. This feature produces the foundational infrastructure that all other features build upon.

- **Depends On**: None

- **Scope**:
  - SDK crate structure: `models.rs` (core entities/DTOs), `api.rs` (trait definitions for `ResourceGroupClient` and `ResourceGroupReadHierarchy`), `error.rs` (`ResourceGroupError` taxonomy)
  - Module lifecycle (`#[modkit::module]`, `Module` trait, ClientHub registration of `ResourceGroupClient` and `ResourceGroupReadHierarchy`)
  - REST API shell (OperationBuilder setup, base path `/api/resource-group/v1/`, OData query infrastructure)
  - DB migration creating all four tables (`resource_group_type`, `resource_group`, `resource_group_closure`, `resource_group_membership`) with indexes and constraints
  - SeaORM entity definitions and repository trait interfaces
  - Unified error mapper (domain/infra failures → stable public error categories)
  - Module initialization order (phased startup for circular dependency resolution with AuthZ)

- **Out of scope**:
  - Domain logic (type validation, forest invariants, closure maintenance) — covered by Features 2–4
  - Integration read routing and MTLS endpoint allowlist — covered by Feature 5

- **Requirements Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-fr-rest-api`
  - [ ] `p1` - `cpt-cf-resource-group-fr-odata-query`
  - [ ] `p1` - `cpt-cf-resource-group-fr-deterministic-errors`
  - [ ] `p1` - `cpt-cf-resource-group-fr-no-authz-and-sql-logic`
  - [ ] `p1` - `cpt-cf-resource-group-nfr-deterministic-errors`
  - [ ] `p1` - `cpt-cf-resource-group-nfr-production-scale`

- **Design Principles Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-principle-policy-agnostic`

- **Design Constraints Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-constraint-no-authz-decision`
  - [ ] `p1` - `cpt-cf-resource-group-constraint-no-sql-filter-generation`
  - [ ] `p1` - `cpt-cf-resource-group-constraint-db-agnostic`

- **Domain Model Entities**:
  - `ResourceGroupType`
  - `ResourceGroup`
  - `ResourceGroupMembership`
  - `ResourceGroupClosure`
  - `ResourceGroupError`

- **Design Components**:
  - [ ] `p1` - `cpt-cf-resource-group-component-module`
  - [ ] `p1` - `cpt-cf-resource-group-component-persistence-adapter`

- **API**:
  - Base path `/api/resource-group/v1/` (OperationBuilder setup)
  - OData query infrastructure (`$filter`, `$top`, `$skip`)

- **Sequences**:
  - `cpt-cf-resource-group-seq-init-order`

- **Data**:
  - `resource_group_type` (schema definition)
  - `resource_group` (schema definition)
  - `resource_group_closure` (schema definition)
  - `resource_group_membership` (schema definition)

---

### 2. Type Management - HIGH

- [ ] `p1` - **ID**: `cpt-cf-resource-group-feature-type-management`

- **Purpose**: Implement the full type lifecycle — CRUD operations, code format validation with case-insensitive normalization, uniqueness enforcement, seed path, and delete-if-unused guard.

- **Depends On**: `cpt-cf-resource-group-feature-domain-foundation`

- **Scope**:
  - Type create with code validation (format, length, case-insensitive normalization)
  - Type update (parent type rules modification)
  - Type get by code
  - Type list with OData filtering on `code`
  - Type delete with usage guard (reject if entities reference the type)
  - Uniqueness enforcement via `code_ci` persistence constraint with deterministic conflict mapping
  - Seed types path (deterministic pre-deployment upsert)
  - REST endpoints: `GET/POST /types`, `GET/PUT/DELETE /types/{code}`

- **Out of scope**:
  - Parent-child type compatibility validation during entity creation — covered by Feature 3
  - Type usage in membership context — covered by Feature 4

- **Requirements Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-fr-manage-types`
  - [ ] `p1` - `cpt-cf-resource-group-fr-validate-type-code`
  - [ ] `p1` - `cpt-cf-resource-group-fr-reject-duplicate-type`
  - [ ] `p1` - `cpt-cf-resource-group-fr-delete-type-only-if-empty`
  - [ ] `p1` - `cpt-cf-resource-group-fr-seed-types`

- **Design Principles Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-principle-dynamic-types`

- **Design Constraints Covered**:
  - None

- **Domain Model Entities**:
  - `ResourceGroupType`

- **Design Components**:
  - [ ] `p1` - `cpt-cf-resource-group-component-type-service`

- **API**:
  - GET /api/resource-group/v1/types
  - POST /api/resource-group/v1/types
  - GET /api/resource-group/v1/types/{code}
  - PUT /api/resource-group/v1/types/{code}
  - DELETE /api/resource-group/v1/types/{code}

- **Sequences**:
  - None

- **Data**:
  - `resource_group_type` (domain logic)

---

### 3. Entity & Hierarchy Management - HIGH

- [ ] `p1` - **ID**: `cpt-cf-resource-group-feature-entity-hierarchy`

- **Purpose**: Implement entity CRUD with strict forest topology enforcement, closure-table hierarchy maintenance, subtree operations (move/delete), depth-based hierarchy queries, and query profile enforcement (max_depth/max_width guardrails).

- **Depends On**: `cpt-cf-resource-group-feature-type-management` (entities reference types for parent-child compatibility)

- **Scope**:
  - Entity create/get/update/move/delete
  - Forest invariant enforcement (single parent, cycle prevention via closure-table check)
  - Parent type compatibility validation (on create, move, and group_type change)
  - Closure table maintenance (self-row, ancestor-descendant rows on create/move/delete)
  - Ancestor/descendant queries ordered by depth
  - Depth-based hierarchy traversal (`list_group_depth` with relative depth)
  - Subtree move (closure recalculation in single SERIALIZABLE transaction)
  - Force delete (cascade subtree and memberships)
  - Delete guard (reject if active references exist, unless force=true)
  - Query profile enforcement (max_depth/max_width on writes)
  - Profile change safety (tightened profiles: full reads, reject violating writes)
  - Seed groups path (deterministic pre-deployment hierarchy creation)
  - REST endpoints: `GET/POST /groups`, `GET/PUT/DELETE /groups/{group_id}`, `GET /groups/{group_id}/depth`

- **Out of scope**:
  - Membership link management — covered by Feature 4
  - Integration read routing and AuthZ interop — covered by Feature 5
  - Type lifecycle management — covered by Feature 2

- **Requirements Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-fr-manage-entities`
  - [ ] `p1` - `cpt-cf-resource-group-fr-enforce-forest-hierarchy`
  - [ ] `p1` - `cpt-cf-resource-group-fr-validate-parent-type`
  - [ ] `p1` - `cpt-cf-resource-group-fr-delete-entity-no-active-references`
  - [ ] `p1` - `cpt-cf-resource-group-fr-closure-table`
  - [ ] `p1` - `cpt-cf-resource-group-fr-query-group-hierarchy`
  - [ ] `p1` - `cpt-cf-resource-group-fr-subtree-operations`
  - [ ] `p1` - `cpt-cf-resource-group-fr-query-profile`
  - [ ] `p1` - `cpt-cf-resource-group-fr-profile-change-no-rewrite`
  - [ ] `p1` - `cpt-cf-resource-group-fr-reduced-constraints-behavior`
  - [ ] `p2` - `cpt-cf-resource-group-fr-force-delete`
  - [ ] `p1` - `cpt-cf-resource-group-fr-list-groups-depth`
  - [ ] `p1` - `cpt-cf-resource-group-fr-seed-groups`
  - [ ] `p1` - `cpt-cf-resource-group-nfr-hierarchy-query-latency`
  - [ ] `p1` - `cpt-cf-resource-group-nfr-transactional-consistency`

- **Design Principles Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-principle-strict-forest`
  - [ ] `p1` - `cpt-cf-resource-group-principle-query-profile-guardrail`

- **Design Constraints Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-constraint-profile-change-safety`

- **Domain Model Entities**:
  - `ResourceGroup`
  - `ResourceGroupClosure`

- **Design Components**:
  - [ ] `p1` - `cpt-cf-resource-group-component-entity-service`
  - [ ] `p1` - `cpt-cf-resource-group-component-hierarchy-service`

- **API**:
  - GET /api/resource-group/v1/groups
  - POST /api/resource-group/v1/groups
  - GET /api/resource-group/v1/groups/{group_id}
  - PUT /api/resource-group/v1/groups/{group_id}
  - DELETE /api/resource-group/v1/groups/{group_id}
  - GET /api/resource-group/v1/groups/{group_id}/depth

- **Sequences**:
  - `cpt-cf-resource-group-seq-create-entity-with-parent`
  - `cpt-cf-resource-group-seq-move-subtree`

- **Data**:
  - `resource_group` (domain logic)
  - `resource_group_closure` (domain logic)

---

### 4. Membership Management - MEDIUM

- [ ] `p2` - **ID**: `cpt-cf-resource-group-feature-membership`

- **Purpose**: Implement membership CRUD with tenant-scoped ownership-graph semantics, seed path, and indexed lookups by group and resource.

- **Depends On**: `cpt-cf-resource-group-feature-entity-hierarchy` (memberships reference groups)

- **Scope**:
  - Add/remove/list membership links
  - Tenant scope validation in ownership-graph profile (caller effective scope vs target group tenant)
  - Active-reference guard (protect entity deletion when memberships exist)
  - Seed memberships path (deterministic pre-deployment membership creation)
  - Reverse lookups (by `resource_type` + `resource_id`)
  - OData filtering on memberships
  - REST endpoints: `GET /memberships`, `POST/DELETE /memberships/{group_id}/{resource_type}/{resource_id}`

- **Out of scope**:
  - Group/entity CRUD — covered by Feature 3
  - Integration read contract exposure — covered by Feature 5

- **Requirements Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-fr-manage-membership`
  - [ ] `p1` - `cpt-cf-resource-group-fr-query-membership-relations`
  - [ ] `p1` - `cpt-cf-resource-group-fr-seed-memberships`
  - [ ] `p1` - `cpt-cf-resource-group-fr-tenant-scope-ownership-graph`
  - [ ] `p1` - `cpt-cf-resource-group-nfr-membership-query-latency`

- **Design Principles Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-principle-tenant-scope-ownership-graph`

- **Design Constraints Covered**:
  - None

- **Domain Model Entities**:
  - `ResourceGroupMembership`

- **Design Components**:
  - [ ] `p1` - `cpt-cf-resource-group-component-membership-service`

- **API**:
  - GET /api/resource-group/v1/memberships
  - POST /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}
  - DELETE /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}

- **Sequences**:
  - None

- **Data**:
  - `resource_group_membership` (domain logic)

---

### 5. Integration Read & AuthZ Interop - MEDIUM

- [ ] `p2` - **ID**: `cpt-cf-resource-group-feature-integration-read`

- **Purpose**: Expose the read-only `ResourceGroupReadHierarchy` contract for AuthZ plugin consumption, implement plugin gateway routing (built-in vs vendor-specific provider), and enforce JWT/MTLS dual authentication with endpoint-level allowlisting.

- **Depends On**: `cpt-cf-resource-group-feature-entity-hierarchy` (hierarchy data must be available for reads)

- **Scope**:
  - `ResourceGroupReadHierarchy` trait implementation (hierarchy-only reads for AuthZ plugin)
  - `ResourceGroupReadPluginClient` trait for vendor-specific provider delegation
  - Plugin gateway routing (built-in provider: local data path; vendor: resolve scoped plugin instance)
  - MTLS authentication path (certificate verification, endpoint allowlist, system SecurityContext)
  - JWT authentication path (AuthZ evaluation via PolicyEnforcer, AccessScope application)
  - MTLS endpoint allowlist enforcement (only `/groups/{id}/depth` reachable via MTLS)
  - Tenant projection rules (hierarchy reads include `tenant_id`; membership reads derive it)
  - Caller identity propagation (`SecurityContext` forwarded without policy interpretation)

- **Out of scope**:
  - AuthZ policy evaluation logic — owned by AuthZ module
  - SQL filter generation — owned by PEP/compiler
  - Hierarchy/membership data mutations — covered by Features 3–4

- **Requirements Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-fr-integration-read-port`
  - [ ] `p1` - `cpt-cf-resource-group-fr-dual-auth-modes`

- **Design Principles Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-principle-policy-agnostic`
  - [ ] `p1` - `cpt-cf-resource-group-principle-tenant-scope-ownership-graph`

- **Design Constraints Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-constraint-no-authz-decision`
  - [ ] `p1` - `cpt-cf-resource-group-constraint-no-sql-filter-generation`

- **Domain Model Entities**:
  - `ResourceGroupWithDepth`

- **Design Components**:
  - [ ] `p1` - `cpt-cf-resource-group-component-integration-read-service`

- **API**:
  - GET /api/resource-group/v1/groups/{group_id}/depth (MTLS path)
  - SDK: `ResourceGroupReadHierarchy.list_group_depth()`
  - SDK: `ResourceGroupReadPluginClient` (plugin delegation)

- **Sequences**:
  - `cpt-cf-resource-group-seq-authz-rg-sql-split`
  - `cpt-cf-resource-group-seq-e2e-authz-flow`
  - `cpt-cf-resource-group-seq-auth-modes`
  - `cpt-cf-resource-group-seq-mtls-authz-read`
  - `cpt-cf-resource-group-seq-jwt-rg-request`

- **Data**:
  - None (reads existing data from Features 1, 3, 4)

---

## 3. Feature Dependencies

```text
cpt-cf-resource-group-feature-domain-foundation
    |
    +---> cpt-cf-resource-group-feature-type-management
              |
              +---> cpt-cf-resource-group-feature-entity-hierarchy
                        |
                        +---> cpt-cf-resource-group-feature-membership
                        |
                        +---> cpt-cf-resource-group-feature-integration-read
```

**Dependency Rationale**:

- `cpt-cf-resource-group-feature-type-management` requires `cpt-cf-resource-group-feature-domain-foundation`: type service needs SDK models, module shell, persistence adapter, and DB schema to operate
- `cpt-cf-resource-group-feature-entity-hierarchy` requires `cpt-cf-resource-group-feature-type-management`: entities reference types for parent-child compatibility validation; types must be created before entities
- `cpt-cf-resource-group-feature-membership` requires `cpt-cf-resource-group-feature-entity-hierarchy`: membership links reference groups that must exist; membership operations validate group existence
- `cpt-cf-resource-group-feature-integration-read` requires `cpt-cf-resource-group-feature-entity-hierarchy`: integration read contract serves hierarchy data that must be populated by entity/hierarchy management
- `cpt-cf-resource-group-feature-membership` and `cpt-cf-resource-group-feature-integration-read` are independent of each other and can be developed in parallel
