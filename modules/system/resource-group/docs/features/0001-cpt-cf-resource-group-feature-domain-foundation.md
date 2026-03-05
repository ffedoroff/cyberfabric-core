# Feature: Domain Foundation

- [ ] `p1` - **ID**: `cpt-cf-resource-group-featstatus-domain-foundation`
- [ ] `p1` - `cpt-cf-resource-group-feature-domain-foundation`

## 1. Feature Context

### 1.1 Overview

Establish the module skeleton for Resource Group (RG): SDK crate with models, trait contracts, and error taxonomy; module lifecycle with ClientHub registration; REST API shell with OperationBuilder and OData infrastructure; database migration for all four tables; SeaORM entity definitions and repository trait interfaces; unified error mapper; and phased module initialization order.

### 1.2 Purpose

This feature produces the foundational infrastructure that all other RG features build upon. Without a working module shell, persistence layer, and SDK contracts, Features 2–5 (Type Management, Entity & Hierarchy, Membership, Integration Read) cannot be implemented.

Addresses:
- `cpt-cf-resource-group-fr-rest-api` — REST API layer with OperationBuilder
- `cpt-cf-resource-group-fr-odata-query` — OData query infrastructure
- `cpt-cf-resource-group-fr-deterministic-errors` — unified error mapper
- `cpt-cf-resource-group-fr-no-authz-and-sql-logic` — policy-agnostic core boundary
- `cpt-cf-resource-group-nfr-deterministic-errors` — stable public error categories
- `cpt-cf-resource-group-nfr-production-scale` — schema design with composite indexes
- `cpt-cf-resource-group-principle-policy-agnostic` — RG handles graph/membership data only

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-instance-administrator` | Deploys module, runs DB migration, configures query profile |
| `cpt-cf-resource-group-actor-apps` | Resolves `ResourceGroupClient` and `ResourceGroupReadHierarchy` from ClientHub after module initialization |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) — `cpt-cf-resource-group-feature-domain-foundation`
- **OpenAPI**: [openapi.yaml](../openapi.yaml) — base path `/api/resource-group/v1/`
- **Migration**: [migration.sql](../migration.sql) — four tables with indexes and constraints
- **Design Components**: `cpt-cf-resource-group-component-module`, `cpt-cf-resource-group-component-persistence-adapter`
- **Design Constraints**: `cpt-cf-resource-group-constraint-no-authz-decision`, `cpt-cf-resource-group-constraint-no-sql-filter-generation`, `cpt-cf-resource-group-constraint-db-agnostic`
- **Dependencies**: None (this is the first feature in the chain)

## 2. Actor Flows (CDSL)

### Module Bootstrap Flow

- [ ] `p1` - **ID**: `cpt-cf-resource-group-flow-module-bootstrap`

**Actor**: `cpt-cf-resource-group-actor-instance-administrator`

**Success Scenarios**:
- DB migration applies all four tables with indexes and constraints
- Module initializes in Phase 1 (SystemCapability), registers SDK clients in ClientHub
- REST API shell becomes available in Phase 2 (ready), accepting traffic on base path `/api/resource-group/v1/`

**Error Scenarios**:
- Migration fails due to pre-existing schema conflict — deterministic error, deployment halts
- ClientHub registration fails — module startup aborts with logged error

**Steps**:
1. [ ] - `p1` - Instance Administrator triggers deployment (migration + module start) - `inst-bootstrap-1`
2. [ ] - `p1` - DB: RUN migration.sql — CREATE TABLE `resource_group_type`, `resource_group`, `resource_group_closure`, `resource_group_membership` with indexes and constraints - `inst-bootstrap-2`
3. [ ] - `p1` - **IF** migration succeeds - `inst-bootstrap-3`
   1. [ ] - `p1` - Module enters Phase 1 (SystemCapability) initialization - `inst-bootstrap-3a`
   2. [ ] - `p1` - Module wires domain services and persistence repositories - `inst-bootstrap-3b`
   3. [ ] - `p1` - Module registers `ResourceGroupClient` in ClientHub - `inst-bootstrap-3c`
   4. [ ] - `p1` - Module registers `ResourceGroupReadHierarchy` in ClientHub - `inst-bootstrap-3d`
   5. [ ] - `p1` - Module loads query profile config (`max_depth`, `max_width`) - `inst-bootstrap-3e`
4. [ ] - `p1` - **ELSE** - `inst-bootstrap-4`
   1. [ ] - `p1` - **RETURN** migration failure — deployment halts - `inst-bootstrap-4a`
5. [ ] - `p1` - Module enters Phase 2 (ready) — REST endpoints start accepting traffic - `inst-bootstrap-5`
6. [ ] - `p1` - **RETURN** module operational — SDK clients available via ClientHub, REST API accepting requests on `/api/resource-group/v1/` - `inst-bootstrap-6`

### SDK Client Resolution Flow

- [ ] `p2` - **ID**: `cpt-cf-resource-group-flow-sdk-client-resolution`

**Actor**: `cpt-cf-resource-group-actor-apps`

**Success Scenarios**:
- App resolves `ResourceGroupClient` from ClientHub and invokes SDK methods
- AuthZ plugin resolves `ResourceGroupReadHierarchy` from ClientHub for hierarchy-only reads

**Error Scenarios**:
- ClientHub resolution fails because RG module has not completed Phase 1 — deterministic `ClientNotFound` error

**Steps**:
1. [ ] - `p2` - App calls `hub.get::<dyn ResourceGroupClient>()` - `inst-resolve-1`
2. [ ] - `p2` - **IF** RG module has completed Phase 1 registration - `inst-resolve-2`
   1. [ ] - `p2` - ClientHub returns `Arc<dyn ResourceGroupClient>` backed by `RgService` - `inst-resolve-2a`
   2. [ ] - `p2` - **RETURN** client reference — app can invoke type/group/membership/hierarchy operations - `inst-resolve-2b`
3. [ ] - `p2` - **ELSE** - `inst-resolve-3`
   1. [ ] - `p2` - **RETURN** `ClientNotFound` error — RG module not yet initialized - `inst-resolve-3a`

## 3. Processes / Business Logic (CDSL)

### Error Mapping Process

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-error-mapping`

**Input**: Domain or infrastructure failure from any RG service or repository

**Output**: `ResourceGroupError` variant — stable public error category

**Steps**:
1. [ ] - `p1` - Receive domain/infra failure from service or repository layer - `inst-errmap-1`
2. [ ] - `p1` - **IF** failure is invalid input (format, length, missing required field) - `inst-errmap-2`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::Validation` with field-level detail - `inst-errmap-2a`
3. [ ] - `p1` - **IF** failure is missing type or entity (lookup returned no rows) - `inst-errmap-3`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::NotFound` with entity kind and identifier - `inst-errmap-3a`
4. [ ] - `p1` - **IF** failure is duplicate type code (unique constraint violation on `code_ci`) - `inst-errmap-4`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::TypeAlreadyExists` with conflicting code - `inst-errmap-4a`
5. [ ] - `p1` - **IF** failure is invalid parent type (parent-child compatibility violation) - `inst-errmap-5`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::InvalidParentType` with type codes involved - `inst-errmap-5a`
6. [ ] - `p1` - **IF** failure is cycle detection (closure table cycle check) - `inst-errmap-6`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::CycleDetected` with node identifiers - `inst-errmap-6a`
7. [ ] - `p1` - **IF** failure is active references on delete (children or memberships exist) - `inst-errmap-7`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::ConflictActiveReferences` with reference count - `inst-errmap-7a`
8. [ ] - `p1` - **IF** failure is depth or width limit violation (query profile exceeded) - `inst-errmap-8`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::LimitViolation` with limit name and values - `inst-errmap-8a`
9. [ ] - `p1` - **IF** failure is tenant-incompatible write (parent/child/membership tenant mismatch) - `inst-errmap-9`
   1. [ ] - `p1` - **RETURN** `ResourceGroupError::TenantIncompatibility` with tenant context - `inst-errmap-9a`
10. [ ] - `p1` - **IF** failure is infrastructure timeout or service unavailability - `inst-errmap-10`
    1. [ ] - `p1` - **RETURN** `ResourceGroupError::ServiceUnavailable` without internal details - `inst-errmap-10a`
11. [ ] - `p1` - **ELSE** (unexpected/unclassified failure) - `inst-errmap-11`
    1. [ ] - `p1` - **RETURN** `ResourceGroupError::Internal` without leaking internal details - `inst-errmap-11a`

### Module Phased Initialization Process

- [ ] `p1` - **ID**: `cpt-cf-resource-group-algo-phased-init`

**Input**: Module registration request from hyperspot-server module orchestrator

**Output**: Module fully initialized with SDK clients in ClientHub and REST endpoints accepting traffic

**Steps**:
1. [ ] - `p1` - Module orchestrator calls `RgModule::init()` during Phase 1 (SystemCapability) - `inst-init-1`
2. [ ] - `p1` - Instantiate persistence repositories (SeaORM entity access, connection pool) - `inst-init-2`
3. [ ] - `p1` - Instantiate domain services (type, entity, hierarchy, membership) with repository dependencies - `inst-init-3`
4. [ ] - `p1` - Instantiate `RgService` (unified service facade implementing `ResourceGroupClient` and `ResourceGroupReadHierarchy`) - `inst-init-4`
5. [ ] - `p1` - Register `Arc<dyn ResourceGroupClient>` in ClientHub (`hub.register::<dyn ResourceGroupClient>(svc.clone())`) - `inst-init-5`
6. [ ] - `p1` - Register `Arc<dyn ResourceGroupReadHierarchy>` in ClientHub (`hub.register::<dyn ResourceGroupReadHierarchy>(svc.clone())`) - `inst-init-6`
7. [ ] - `p1` - **RETURN** Phase 1 complete — SDK clients available, REST endpoints NOT yet accepting traffic - `inst-init-7`
8. [ ] - `p1` - Module orchestrator signals Phase 2 (ready) — AuthZ Resolver has completed init (step 2 in `cpt-cf-resource-group-seq-init-order`) - `inst-init-8`
9. [ ] - `p1` - Configure REST routes via OperationBuilder (base path `/api/resource-group/v1/`, OData parameters, handlers) - `inst-init-9`
10. [ ] - `p1` - REST endpoints start accepting traffic — write operations now route through PolicyEnforcer → AuthZResolverClient - `inst-init-10`
11. [ ] - `p1` - **RETURN** Phase 2 complete — module fully operational - `inst-init-11`

## 4. States (CDSL)

Not applicable. Domain Foundation establishes infrastructure only. Entity lifecycle states (if any) are defined in Features 2–5 where domain logic operates.

## 5. Definitions of Done

### SDK Crate Structure

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-crate`

The system **MUST** provide an SDK crate (`resource-group-sdk`) with three modules: `models.rs` defining all SDK models and DTOs (`ResourceGroupType`, `ResourceGroup`, `ResourceGroupWithDepth`, `ResourceGroupMembership`, `CreateTypeRequest`, `UpdateTypeRequest`, `CreateGroupRequest`, `UpdateGroupRequest`, `AddMembershipRequest`, `RemoveMembershipRequest`, `Page<T>`, `PageInfo`); `api.rs` defining the `ResourceGroupClient` and `ResourceGroupReadHierarchy` trait contracts with `async_trait` and `SecurityContext`; `error.rs` defining the `ResourceGroupError` enum with all ten public error variants (`Validation`, `NotFound`, `TypeAlreadyExists`, `InvalidParentType`, `CycleDetected`, `ConflictActiveReferences`, `LimitViolation`, `TenantIncompatibility`, `ServiceUnavailable`, `Internal`).

**Implements**:
- `cpt-cf-resource-group-flow-sdk-client-resolution`
- `cpt-cf-resource-group-algo-error-mapping`

**Touches**:
- Entities: `ResourceGroupType`, `ResourceGroup`, `ResourceGroupWithDepth`, `ResourceGroupMembership`, `ResourceGroupError`

### Module Lifecycle and ClientHub Registration

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-module-lifecycle`

The system **MUST** implement the `#[modkit::module]` macro and `Module` trait for the RG module. The module **MUST** register `Arc<dyn ResourceGroupClient>` and `Arc<dyn ResourceGroupReadHierarchy>` in ClientHub during Phase 1 (SystemCapability). Both registrations **MUST** be backed by a single `RgService` instance. The module **MUST** follow the phased initialization order defined in `cpt-cf-resource-group-seq-init-order` to resolve the circular dependency with AuthZ.

**Implements**:
- `cpt-cf-resource-group-flow-module-bootstrap`
- `cpt-cf-resource-group-algo-phased-init`

**Touches**:
- Entities: `RgService`, `RgModule`

### REST API Shell

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-rest-api-shell`

The system **MUST** configure REST endpoint routes via OperationBuilder under base path `/api/resource-group/v1/`. The shell **MUST** wire all 14 REST endpoints defined in the OpenAPI contract (`listTypes`, `createType`, `getType`, `updateType`, `deleteType`, `listGroups`, `createGroup`, `getGroup`, `updateGroup`, `deleteGroup`, `listGroupDepth`, `listMemberships`, `addMembership`, `deleteMembership`). Each handler **MUST** accept OData query parameters (`$filter`, `$top`, `$skip`) on list endpoints. Handler implementations at this stage return stub responses or delegate to domain services wired in Phase 1. OData field-to-column mapping infrastructure **MUST** be configured for all filterable fields per OpenAPI `x-odata-filter` definitions.

**Implements**:
- `cpt-cf-resource-group-flow-module-bootstrap`

**Touches**:
- API: all endpoints under `/api/resource-group/v1/`

### Database Migration

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-db-migration`

The system **MUST** provide a database migration that creates four tables: `resource_group_type` (TEXT PK `code`, TEXT[] `parents`, timestamps), `resource_group` (UUID PK `id`, FK to `resource_group_type.code`, FK to self `parent_id`, `tenant_id`, `external_id`, timestamps), `resource_group_closure` (composite PK `ancestor_id`/`descendant_id`, `depth`, FKs to `resource_group`), `resource_group_membership` (composite unique `group_id`/`resource_type`/`resource_id`, FK to `resource_group`, timestamp). The migration **MUST** create all indexes defined in `migration.sql`: case-insensitive unique index on `resource_group_type.code`, parent/name/external_id/group_type indexes on `resource_group`, descendant and ancestor+depth indexes on `resource_group_closure`, resource_type+resource_id index on `resource_group_membership`. All foreign keys **MUST** use `ON UPDATE CASCADE` / `ON DELETE RESTRICT`. The migration **MUST NOT** use vendor-specific SQL extensions per `cpt-cf-resource-group-constraint-db-agnostic`.

**Implements**:
- `cpt-cf-resource-group-flow-module-bootstrap`

**Touches**:
- DB: `resource_group_type`, `resource_group`, `resource_group_closure`, `resource_group_membership`

### SeaORM Entities and Repository Interfaces

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-seaorm-entities`

The system **MUST** define SeaORM entity structs for all four tables matching the migration schema. The system **MUST** define repository trait interfaces for type, group, closure, and membership persistence operations. Repository traits **MUST** accept transaction-scoped connections (`SecureTx` / `DBRunner`) to support transactional consistency per `cpt-cf-resource-group-nfr-transactional-consistency`. Repository implementations **MUST** use parameterized queries for all operations.

**Implements**:
- `cpt-cf-resource-group-flow-module-bootstrap`

**Touches**:
- DB: `resource_group_type`, `resource_group`, `resource_group_closure`, `resource_group_membership`
- Entities: SeaORM entity definitions

### Unified Error Mapper

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-error-mapper`

The system **MUST** implement a unified error mapper that translates all domain and infrastructure failures into stable `ResourceGroupError` public categories per the error mapping table in DESIGN section 3.9. The mapper **MUST** cover all ten failure-to-error mappings: invalid input → `Validation`, missing type/entity → `NotFound`, duplicate type → `TypeAlreadyExists`, invalid parent type → `InvalidParentType`, cycle attempt → `CycleDetected`, active references on delete → `ConflictActiveReferences`, depth/width violation → `LimitViolation`, tenant-incompatible write → `TenantIncompatibility`, infra timeout → `ServiceUnavailable`, unexpected failure → `Internal`. Error responses **MUST** map to RFC 9457 Problem format for REST API responses. Internal details **MUST NOT** leak in `ServiceUnavailable` or `Internal` error responses.

**Implements**:
- `cpt-cf-resource-group-algo-error-mapping`

**Touches**:
- Entities: `ResourceGroupError`

### Module Initialization Order

- [ ] `p1` - **ID**: `cpt-cf-resource-group-dod-init-order`

The system **MUST** implement phased module initialization per `cpt-cf-resource-group-seq-init-order`. Phase 1 (SystemCapability): RG Module registers `ResourceGroupClient` and `ResourceGroupReadHierarchy` in ClientHub; REST endpoints are NOT yet accepting traffic. Phase 2 (ready): RG Module starts accepting REST traffic after AuthZ Resolver has completed its init and registered `AuthZResolverClient`. There **MUST** be no deadlock: RG registers read clients before AuthZ initializes, and AuthZ registers its client before RG starts accepting write traffic.

**Implements**:
- `cpt-cf-resource-group-algo-phased-init`

**Touches**:
- Entities: `RgModule`, `RgService`

## 6. Acceptance Criteria

- [ ] SDK crate compiles with `models.rs`, `api.rs`, `error.rs` exposing all defined types, traits, and error variants
- [ ] `ResourceGroupClient` trait defines all 14 methods matching the OpenAPI contract
- [ ] `ResourceGroupReadHierarchy` trait defines `list_group_depth` method
- [ ] `ResourceGroupError` enum has exactly ten variants matching DESIGN section 3.9 error mapping
- [ ] DB migration applies cleanly on a fresh database creating all four tables with correct schemas, constraints, and indexes
- [ ] DB migration is idempotent — re-running does not produce errors or duplicate objects
- [ ] SeaORM entities compile and map to all four tables with correct column types
- [ ] Repository traits define CRUD operations for all four tables accepting transaction-scoped connections
- [ ] Module initializes in Phase 1, registers both SDK clients in ClientHub
- [ ] `hub.get::<dyn ResourceGroupClient>()` succeeds after Phase 1 init
- [ ] `hub.get::<dyn ResourceGroupReadHierarchy>()` succeeds after Phase 1 init
- [ ] REST API shell responds on base path `/api/resource-group/v1/` after Phase 2
- [ ] OData query parameters (`$filter`, `$top`, `$skip`) are accepted on all list endpoints
- [ ] Error mapper converts each of the ten failure categories to correct `ResourceGroupError` variant
- [ ] RFC 9457 Problem responses do not leak internal stack traces or infrastructure details
- [ ] No vendor-specific SQL in migration (`cpt-cf-resource-group-constraint-db-agnostic`)
- [ ] Module does not implement AuthZ decision logic or SQL filter generation (`cpt-cf-resource-group-constraint-no-authz-decision`, `cpt-cf-resource-group-constraint-no-sql-filter-generation`)

## 7. Non-Applicable Domains

- **States (CDSL)**: Not applicable — Domain Foundation establishes infrastructure only; no entity lifecycle state machines are introduced at this layer. Entity states (if any) belong to Features 2–5.
- **Usability (UX)**: Not applicable — RG is a backend infrastructure module with no frontend or user-facing UI.
- **Compliance (COMPL)**: Not applicable — compliance controls are platform-level; RG does not own regulated data directly.
- **Performance**: Addressed implicitly through index design in migration.sql; explicit performance flows belong to Features 3–5 where query-heavy operations execute.
- **Security (AuthN/AuthZ)**: Authentication mode wiring (JWT/MTLS) and AuthZ PolicyEnforcer integration are exercised at this layer via REST handler setup, but detailed security flows belong to Feature 5 (Integration Read). Error mapper ensures no information leakage in error responses.
