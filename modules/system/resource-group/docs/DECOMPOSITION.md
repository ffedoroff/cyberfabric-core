# Decomposition: Resource Group (RG)

- [ ] `p1` - **ID**: `cpt-cf-resource-group-status-overall`

## 1. Overview

The Resource Group (RG) DESIGN is decomposed into six features organized as a linear chain with a terminal fork.

**Decomposition strategy**: Features follow the natural dependency order of the RG architecture layers — persistence/SDK foundation first, then domain services (types → entities/hierarchy → memberships), then authorization enforcement, and finally the integration read contract for MTLS/plugin interop. Each feature maps to one or two DESIGN components with high cohesion and minimal cross-feature coupling.

| # | Feature | Priority | Components | Key Concern |
|---|---------|----------|------------|-------------|
| 1 | Domain Foundation | HIGH | module, persistence-adapter | SDK, module shell, DB, errors |
| 2 | Type Management | HIGH | type-service | Type lifecycle and validation |
| 3 | Entity & Hierarchy | HIGH | entity-service, hierarchy-service | Forest topology, closure table |
| 4 | Membership Management | MEDIUM | membership-service | Membership CRUD, tenant scope |
| 5 | AuthZ Enforcement | HIGH | module, all services | PolicyEnforcer, AccessScope, tenant isolation |
| 6 | MTLS Auth & Plugin Gateway | MEDIUM | integration-read-service | MTLS auth, plugin gateway (DEFERRED) |
| 7 | AuthZ Advanced Constraint Types | HIGH | authz-resolver-sdk, modkit-db, static-authz-plugin | `in_tenant_subtree`, `in_group`, `in_group_subtree` (DONE) |

---

## 2. Entries

### 1. Domain Foundation - HIGH

- [x] `p1` - **ID**: `cpt-cf-resource-group-feature-domain-foundation`

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
  - Integration read routing and MTLS endpoint allowlist — covered by Feature 6
  - PolicyEnforcer integration and AccessScope — covered by Feature 5

- **Requirements Covered**:
  - [x] `p1` - `cpt-cf-resource-group-fr-rest-api`
  - [x] `p1` - `cpt-cf-resource-group-fr-odata-query`
  - [x] `p1` - `cpt-cf-resource-group-fr-deterministic-errors`
  - [x] `p1` - `cpt-cf-resource-group-fr-no-authz-and-sql-logic`
  - [x] `p1` - `cpt-cf-resource-group-nfr-deterministic-errors`
  - [x] `p1` - `cpt-cf-resource-group-nfr-production-scale`

- **Design Principles Covered**:
  - [x] `p1` - `cpt-cf-resource-group-principle-policy-agnostic`

- **Design Constraints Covered**:
  - [x] `p1` - `cpt-cf-resource-group-constraint-no-authz-decision`
  - [x] `p1` - `cpt-cf-resource-group-constraint-no-sql-filter-generation`
  - [x] `p1` - `cpt-cf-resource-group-constraint-db-agnostic`

- **Domain Model Entities**:
  - `ResourceGroupType`
  - `ResourceGroup`
  - `ResourceGroupMembership`
  - `ResourceGroupClosure`
  - `ResourceGroupError`

- **Design Components**:
  - [x] `p1` - `cpt-cf-resource-group-component-module`
  - [x] `p1` - `cpt-cf-resource-group-component-persistence-adapter`

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

- [x] `p1` - **ID**: `cpt-cf-resource-group-feature-type-management`

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
  - [x] `p1` - `cpt-cf-resource-group-fr-manage-types`
  - [x] `p1` - `cpt-cf-resource-group-fr-validate-type-code`
  - [x] `p1` - `cpt-cf-resource-group-fr-reject-duplicate-type`
  - [x] `p1` - `cpt-cf-resource-group-fr-delete-type-only-if-empty`
  - [x] `p1` - `cpt-cf-resource-group-fr-seed-types`

- **Design Principles Covered**:
  - [x] `p1` - `cpt-cf-resource-group-principle-dynamic-types`

- **Design Constraints Covered**:
  - None

- **Domain Model Entities**:
  - `ResourceGroupType`

- **Design Components**:
  - [x] `p1` - `cpt-cf-resource-group-component-type-service`

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

- [x] `p1` - **ID**: `cpt-cf-resource-group-feature-entity-hierarchy`

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
  - Integration read routing and MTLS — covered by Feature 6
  - AuthZ enforcement — covered by Feature 5
  - Type lifecycle management — covered by Feature 2

- **Requirements Covered**:
  - [x] `p1` - `cpt-cf-resource-group-fr-manage-entities`
  - [x] `p1` - `cpt-cf-resource-group-fr-enforce-forest-hierarchy`
  - [x] `p1` - `cpt-cf-resource-group-fr-validate-parent-type`
  - [x] `p1` - `cpt-cf-resource-group-fr-delete-entity-no-active-references`
  - [x] `p1` - `cpt-cf-resource-group-fr-closure-table`
  - [x] `p1` - `cpt-cf-resource-group-fr-query-group-hierarchy`
  - [x] `p1` - `cpt-cf-resource-group-fr-subtree-operations`
  - [x] `p1` - `cpt-cf-resource-group-fr-query-profile`
  - [x] `p1` - `cpt-cf-resource-group-fr-profile-change-no-rewrite`
  - [x] `p1` - `cpt-cf-resource-group-fr-reduced-constraints-behavior`
  - [x] `p2` - `cpt-cf-resource-group-fr-force-delete`
  - [x] `p1` - `cpt-cf-resource-group-fr-list-groups-depth`
  - [x] `p1` - `cpt-cf-resource-group-fr-seed-groups`
  - [x] `p1` - `cpt-cf-resource-group-nfr-hierarchy-query-latency`
  - [x] `p1` - `cpt-cf-resource-group-nfr-transactional-consistency`

- **Design Principles Covered**:
  - [x] `p1` - `cpt-cf-resource-group-principle-strict-forest`
  - [x] `p1` - `cpt-cf-resource-group-principle-query-profile-guardrail`

- **Design Constraints Covered**:
  - [x] `p1` - `cpt-cf-resource-group-constraint-profile-change-safety`

- **Domain Model Entities**:
  - `ResourceGroup`
  - `ResourceGroupClosure`

- **Design Components**:
  - [x] `p1` - `cpt-cf-resource-group-component-entity-service`
  - [x] `p1` - `cpt-cf-resource-group-component-hierarchy-service`

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

- [x] `p2` - **ID**: `cpt-cf-resource-group-feature-membership`

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
  - Integration read contract exposure — covered by Feature 6
  - AuthZ enforcement — covered by Feature 5

- **Requirements Covered**:
  - [x] `p1` - `cpt-cf-resource-group-fr-manage-membership`
  - [x] `p1` - `cpt-cf-resource-group-fr-query-membership-relations`
  - [x] `p1` - `cpt-cf-resource-group-fr-seed-memberships`
  - [x] `p1` - `cpt-cf-resource-group-fr-tenant-scope-ownership-graph`
  - [x] `p1` - `cpt-cf-resource-group-nfr-membership-query-latency`

- **Design Principles Covered**:
  - [x] `p1` - `cpt-cf-resource-group-principle-tenant-scope-ownership-graph`

- **Design Constraints Covered**:
  - None

- **Domain Model Entities**:
  - `ResourceGroupMembership`

- **Design Components**:
  - [x] `p1` - `cpt-cf-resource-group-component-membership-service`

- **API**:
  - GET /api/resource-group/v1/memberships
  - POST /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}
  - DELETE /api/resource-group/v1/memberships/{group_id}/{resource_type}/{resource_id}

- **Sequences**:
  - None

- **Data**:
  - `resource_group_membership` (domain logic)

---

### 5. AuthZ Enforcement - HIGH

- [x] `p1` - **ID**: `cpt-cf-resource-group-feature-authz-enforcement`

- **Purpose**: Integrate PolicyEnforcer into all RG REST handlers to enforce authorization on every request. Group and membership endpoints receive tenant-scoped `AccessScope`; type endpoints require authentication but operate on global data. All repository queries execute through SecureORM with the resolved `AccessScope`.

- **Depends On**: `cpt-cf-resource-group-feature-membership` (all domain features must be implemented before adding authorization layer)

- **Scope**:
  - PolicyEnforcer instantiation in module init (resolve `AuthZResolverClient` from ClientHub)
  - `ResourceType` descriptor definitions for groups and types
  - `access_scope()` calls in all REST handlers before domain logic
  - `AccessScope` propagation through domain services to repository queries
  - SecureORM tenant scoping on group and membership queries
  - EnforcerError to HTTP response mapping (403, 503, 500)
  - Type endpoints: authorized but no tenant constraints (global resource)
  - In-process `ResourceGroupReadHierarchy` access with system `SecurityContext` (AuthZ plugin bypass)

- **Out of scope**:
  - MTLS authentication path — covered by Feature 6
  - Plugin gateway routing — covered by Feature 6
  - AuthZ policy logic — owned by AuthZ module
  - SQL filter generation — owned by PEP/compiler (SecureORM)

- **Requirements Covered**:
  - [x] `p1` - `cpt-cf-resource-group-fr-dual-auth-modes` (JWT path only; MTLS path deferred to Feature 6)

- **Design Principles Covered**:
  - [x] `p1` - `cpt-cf-resource-group-principle-policy-agnostic`

- **Design Constraints Covered**:
  - [x] `p1` - `cpt-cf-resource-group-constraint-no-authz-decision`
  - [x] `p1` - `cpt-cf-resource-group-constraint-no-sql-filter-generation`

- **Domain Model Entities**:
  - None (uses existing entities from Features 1-4)

- **Design Components**:
  - [x] `p1` - `cpt-cf-resource-group-component-module` (enhanced with PolicyEnforcer)

- **API**:
  - All existing endpoints (authorization layer added)

- **Sequences**:
  - `cpt-cf-resource-group-seq-jwt-rg-request`
  - `cpt-cf-resource-group-seq-e2e-authz-flow`

- **Data**:
  - None (queries existing data with AccessScope filtering)

---

### 6. MTLS Auth & Plugin Gateway - MEDIUM (DEFERRED)

- [ ] `p2` - **ID**: `cpt-cf-resource-group-feature-mtls-plugin-gateway`

**Status**: DEFERRED — blocked on platform MTLS infrastructure and plugin architecture readiness

- **Purpose**: Add MTLS authentication path for out-of-process AuthZ plugin consumption, implement plugin gateway routing (built-in vs vendor-specific provider), and enforce MTLS endpoint allowlisting.

- **Depends On**: `cpt-cf-resource-group-feature-authz-enforcement`, `cpt-cf-resource-group-feature-authz-constraint-types`

- **Scope**:
  - MTLS authentication path (certificate verification, endpoint allowlist, system SecurityContext)
  - MTLS endpoint allowlist enforcement (only `/groups/{id}/depth` reachable via MTLS)
  - `ResourceGroupReadPluginClient` trait for vendor-specific provider delegation
  - Plugin gateway routing (built-in provider: local data path; vendor: resolve scoped plugin instance)

- **Out of scope**:
  - JWT auth path — done in Feature 5
  - In-process ClientHub path — done in Feature 5
  - Advanced constraint types — done in Feature 7
  - AuthZ policy evaluation — owned by AuthZ module

- **Requirements Covered**:
  - [ ] `p1` - `cpt-cf-resource-group-fr-integration-read-port`
  - [ ] `p1` - `cpt-cf-resource-group-fr-dual-auth-modes` (MTLS path)

- **Design Components**:
  - [ ] `p1` - `cpt-cf-resource-group-component-integration-read-service`

- **API**:
  - GET /api/resource-group/v1/groups/{group_id}/depth (MTLS path)
  - SDK: `ResourceGroupReadPluginClient` (plugin delegation)

- **Data**:
  - None (reads existing data from Features 1, 3, 4)

---

### 7. AuthZ Advanced Constraint Types - HIGH

- [x] `p1` - **ID**: `cpt-cf-resource-group-feature-authz-constraint-types`

- **Purpose**: Extend the constraint model with `in_tenant_subtree`, `in_group`, `in_group_subtree` predicate types. Implement PEP compiler and SecureORM support for SQL subquery generation using local projection tables. Enhance static-authz-plugin to return advanced predicates based on PEP capabilities.

- **Depends On**: `cpt-cf-resource-group-feature-authz-enforcement` (PolicyEnforcer pipeline must exist)

- **Scope**:
  - New predicate types in `authz-resolver-sdk/constraints.rs`
  - Constraint compiler extension in `authz-resolver-sdk/pep/compiler.rs`
  - SecureORM subquery filter support in `modkit-db/secure/`
  - Static-authz-plugin: return `InTenantSubtree` / `InGroupSubtree` when capabilities declared
  - PEP capability declaration in PolicyEnforcer calls

- **Out of scope**:
  - MTLS authentication — covered by Feature 6
  - Plugin gateway routing — covered by Feature 6
  - Local projection sync — future infrastructure work

- **Requirements Covered**:
  - [x] `p1` - Authorization architecture predicate types (DESIGN.md §Predicate Types Reference)

- **Design Principles Covered**:
  - [x] `p1` - `cpt-cf-resource-group-principle-policy-agnostic`

- **Design Constraints Covered**:
  - [x] `p1` - `cpt-cf-resource-group-constraint-no-authz-decision`
  - [x] `p1` - `cpt-cf-resource-group-constraint-no-sql-filter-generation`

- **Domain Model Entities**:
  - `InTenantSubtreePredicate`, `InGroupPredicate`, `InGroupSubtreePredicate`

- **Design Components**:
  - `authz-resolver-sdk` constraints and compiler
  - `modkit-db` SecureORM scope filters
  - `static-authz-plugin` service

- **Data**:
  - Local projections: `tenant_closure`, `resource_group_closure`, `resource_group_membership`

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
                                  +---> cpt-cf-resource-group-feature-authz-enforcement
                                            |
                                            +---> cpt-cf-resource-group-feature-authz-constraint-types
                                            |         |
                                            |         +---> cpt-cf-resource-group-feature-mtls-plugin-gateway (DEFERRED)
                                            |
                                            +---> cpt-cf-resource-group-feature-mtls-plugin-gateway (DEFERRED)
```

**Dependency Rationale**:

- `cpt-cf-resource-group-feature-type-management` requires `cpt-cf-resource-group-feature-domain-foundation`: type service needs SDK models, module shell, persistence adapter, and DB schema to operate
- `cpt-cf-resource-group-feature-entity-hierarchy` requires `cpt-cf-resource-group-feature-type-management`: entities reference types for parent-child compatibility validation; types must be created before entities
- `cpt-cf-resource-group-feature-membership` requires `cpt-cf-resource-group-feature-entity-hierarchy`: membership links reference groups that must exist; membership operations validate group existence
- `cpt-cf-resource-group-feature-authz-enforcement` requires `cpt-cf-resource-group-feature-membership`: all domain features must be implemented before adding the authorization layer; PolicyEnforcer integration touches all service and handler code
- `cpt-cf-resource-group-feature-authz-constraint-types` requires `cpt-cf-resource-group-feature-authz-enforcement`: advanced predicates build on the PolicyEnforcer pipeline established by Feature 5
- `cpt-cf-resource-group-feature-mtls-plugin-gateway` requires Features 5 and 7: MTLS transport and plugin routing build on top of established JWT + constraints pipeline; DEFERRED pending platform MTLS infrastructure
