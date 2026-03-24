# Feature: SDK Contracts, Error Types & Module Foundation

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-sdk-module-foundation`

- [x] `p1` - `cpt-cf-resource-group-feature-sdk-module-foundation`

<!-- toc -->

- [1. Feature Context](#1-feature-context)
  - [1.1 Overview](#11-overview)
  - [1.2 Purpose](#12-purpose)
  - [1.3 Actors](#13-actors)
  - [1.4 References](#14-references)
- [2. Actor Flows (CDSL)](#2-actor-flows-cdsl)
- [3. Processes / Business Logic (CDSL)](#3-processes--business-logic-cdsl)
  - [GTS Type Path Validation](#gts-type-path-validation)
  - [Domain Error to Problem Mapping](#domain-error-to-problem-mapping)
- [4. States (CDSL)](#4-states-cdsl)
- [5. Definitions of Done](#5-definitions-of-done)
  - [SDK Models and Value Objects](#sdk-models-and-value-objects)
  - [SDK Trait Contracts](#sdk-trait-contracts)
  - [SDK Error Taxonomy](#sdk-error-taxonomy)
  - [Persistence Adapter and DB Migrations](#persistence-adapter-and-db-migrations)
  - [Module Scaffold and Initialization](#module-scaffold-and-initialization)
  - [REST and OData Infrastructure](#rest-and-odata-infrastructure)
- [6. Acceptance Criteria](#6-acceptance-criteria)

<!-- /toc -->

## 1. Feature Context

### 1.1 Overview

Establish the SDK crate with trait contracts, domain models, and error taxonomy; scaffold the RG module with ClientHub registration; implement persistence adapter with SeaORM entities and DB migrations for all 6 tables; wire cross-cutting infrastructure for REST/OData endpoints and deterministic error mapping.

### 1.2 Purpose

This feature provides the foundation that all subsequent RG features depend on. It defines the public API surface (SDK traits), the domain model types, the error taxonomy, and the infrastructure scaffolding (module wiring, persistence, REST framework) without implementing domain-specific business logic.

**Requirements**: `cpt-cf-resource-group-fr-rest-api`, `cpt-cf-resource-group-fr-odata-query`, `cpt-cf-resource-group-fr-deterministic-errors`, `cpt-cf-resource-group-fr-no-authz-and-sql-logic`, `cpt-cf-resource-group-nfr-deterministic-errors`, `cpt-cf-resource-group-nfr-compatibility`, `cpt-cf-resource-group-nfr-production-scale`, `cpt-cf-resource-group-nfr-transactional-consistency`, `cpt-cf-resource-group-interface-resource-group-client`, `cpt-cf-resource-group-interface-integration-read-hierarchy`

**Principles**: `cpt-cf-resource-group-principle-policy-agnostic`

**Constraints**: `cpt-cf-resource-group-constraint-no-authz-decision`, `cpt-cf-resource-group-constraint-no-sql-filter-generation`, `cpt-cf-resource-group-constraint-db-agnostic`, `cpt-cf-resource-group-constraint-surrogate-ids-internal`

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| `cpt-cf-resource-group-actor-apps` | Programmatic consumer of SDK traits via ClientHub |
| `cpt-cf-resource-group-actor-instance-administrator` | Operates migrations and module deployment |

### 1.4 References

- **PRD**: [PRD.md](../PRD.md)
- **Design**: [DESIGN.md](../DESIGN.md)
- **DECOMPOSITION**: [DECOMPOSITION.md](../DECOMPOSITION.md) entry 2.1
- **Dependencies**: None (foundation feature)

## 2. Actor Flows (CDSL)

Not applicable. This feature provides SDK contracts and module infrastructure without user-facing interactions. Actor flows that exercise these contracts are defined in features 2-5 (type management, entity/hierarchy, membership, integration) which implement domain operations on top of this foundation.

## 3. Processes / Business Logic (CDSL)

### GTS Type Path Validation

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-sdk-foundation-validate-gts-type-path`

**Input**: Raw string candidate for a GTS type path

**Output**: Validated `GtsTypePath` value object or validation error

**Steps**:
1. [x] - `p1` - Receive raw string input - `inst-gts-val-1`
2. [x] - `p1` - Trim whitespace and normalize to lowercase - `inst-gts-val-2`
3. [x] - `p1` - **IF** string is empty - `inst-gts-val-3`
   1. [x] - `p1` - **RETURN** Validation error: "GTS type path must not be empty" - `inst-gts-val-3a`
4. [x] - `p1` - **IF** string does not match pattern `^gts\.[a-z0-9_.]+~([a-z0-9_.]+~)*$` - `inst-gts-val-4`
   1. [x] - `p1` - **RETURN** Validation error: "Invalid GTS type path format" - `inst-gts-val-4a`
5. [x] - `p1` - **IF** string length exceeds maximum (255 chars) - `inst-gts-val-5`
   1. [x] - `p1` - **RETURN** Validation error: "GTS type path exceeds maximum length" - `inst-gts-val-5a`
6. [x] - `p1` - Construct `GtsTypePath` value object wrapping the validated string - `inst-gts-val-6`
7. [x] - `p1` - **RETURN** validated `GtsTypePath` - `inst-gts-val-7`

### Domain Error to Problem Mapping

- [x] `p1` - **ID**: `cpt-cf-resource-group-algo-sdk-foundation-map-domain-error`

**Input**: `ResourceGroupError` domain error variant

**Output**: RFC-9457 Problem response with HTTP status, type URI, title, and detail

**Steps**:
1. [x] - `p1` - Receive `ResourceGroupError` variant - `inst-err-map-1`
2. [x] - `p1` - Match error variant to HTTP status and Problem fields - `inst-err-map-2`
   1. [x] - `p1` - `Validation` -> 400 Bad Request, type "validation", field-level details - `inst-err-map-2a`
   2. [x] - `p1` - `NotFound` -> 404 Not Found, type "not-found", entity identifier in detail - `inst-err-map-2b`
   3. [x] - `p1` - `TypeAlreadyExists` -> 409 Conflict, type "type-already-exists", conflicting code in detail - `inst-err-map-2c`
   4. [x] - `p1` - `InvalidParentType` -> 409 Conflict, type "invalid-parent-type", type mismatch in detail - `inst-err-map-2d`
   5. [x] - `p1` - `AllowedParentsViolation` -> 409 Conflict, type "allowed-parents-violation", violating groups in detail - `inst-err-map-2e`
   6. [x] - `p1` - `CycleDetected` -> 409 Conflict, type "cycle-detected", involved node IDs in detail - `inst-err-map-2f`
   7. [x] - `p1` - `ConflictActiveReferences` -> 409 Conflict, type "active-references", reference count in detail - `inst-err-map-2g`
   8. [x] - `p1` - `LimitViolation` -> 409 Conflict, type "limit-violation", limit name and values in detail - `inst-err-map-2h`
   9. [x] - `p1` - `TenantIncompatibility` -> 409 Conflict, type "tenant-incompatibility", tenant IDs in detail - `inst-err-map-2i`
   10. [x] - `p1` - `ServiceUnavailable` -> 503 Service Unavailable, type "service-unavailable" - `inst-err-map-2j`
   11. [x] - `p1` - `Internal` -> 500 Internal Server Error, type "internal", no internal details exposed - `inst-err-map-2k`
3. [x] - `p1` - Construct Problem response with `type`, `title`, `status`, `detail` fields - `inst-err-map-3`
4. [x] - `p1` - **RETURN** Problem response - `inst-err-map-4`

## 4. States (CDSL)

Not applicable. This feature defines SDK contracts, module scaffold, and persistence infrastructure. No entity lifecycle state machines are introduced here. Entity state management is covered by features 2-4 (type/group/membership lifecycle).

## 5. Definitions of Done

### SDK Models and Value Objects

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-foundation-sdk-models`

The system **MUST** define SDK model types in `resource-group-sdk/src/models.rs` that represent the public API surface for all RG domain entities and query constructs.

**Required types**:
- `ResourceGroupType` — type definition with `schema_id` (GtsTypePath), `allowed_parents` (Vec), `allowed_memberships` (Vec), `can_be_root` (bool), `metadata_schema` (Option)
- `ResourceGroup` — group entity with `id` (Uuid), `type` (GtsTypePath), `name` (String), `metadata` (Option), `hierarchy` (ResourceGroupHierarchy with `parent_id`, `tenant_id`)
- `ResourceGroupWithDepth` — extends ResourceGroup with `hierarchy.depth` (i32, relative distance)
- `ResourceGroupMembership` — membership link with `group_id` (Uuid), `resource_type` (GtsTypePath), `resource_id` (String)
- `GtsTypePath` — validated value object wrapping GTS type path string, with format validation
- `Page<T>` — cursor-based pagination wrapper with `items` (Vec), `page_info` (PageInfo)
- `PageInfo` — pagination metadata with `has_next_page`, `has_previous_page`, `start_cursor`, `end_cursor`
- `ListQuery` — OData filter + pagination parameters

**Implements**:
- `cpt-cf-resource-group-algo-sdk-foundation-validate-gts-type-path`

**Constraints**: `cpt-cf-resource-group-constraint-surrogate-ids-internal`

**Touches**:
- Entities: `ResourceGroupType`, `ResourceGroup`, `ResourceGroupWithDepth`, `ResourceGroupMembership`, `GtsTypePath`, `Page`, `PageInfo`, `ListQuery`

### SDK Trait Contracts

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-foundation-sdk-traits`

The system **MUST** define SDK trait contracts in `resource-group-sdk/src/api.rs` that represent the stable public interface for all RG operations.

**Required traits**:
- `ResourceGroupClient` — full CRUD trait: type management (`create_type`, `get_type`, `list_types`, `update_type`, `delete_type`), group management (`create_group`, `get_group`, `list_groups`, `update_group`, `delete_group`, `list_group_depth`), membership management (`add_membership`, `remove_membership`, `list_memberships`). All methods accept `SecurityContext` as first argument.
- `ResourceGroupReadHierarchy` — narrow hierarchy-only read trait: `list_group_depth(ctx, group_id, query)` returning `Page<ResourceGroupWithDepth>`. Used exclusively by AuthZ plugin.
- `ResourceGroupReadPluginClient` — extends `ResourceGroupReadHierarchy` with `list_memberships`. Used for vendor-specific plugin gateway routing.

**Constraints**: `cpt-cf-resource-group-constraint-no-authz-decision`, `cpt-cf-resource-group-constraint-no-sql-filter-generation`

**Touches**:
- Entities: `ResourceGroupClient`, `ResourceGroupReadHierarchy`, `ResourceGroupReadPluginClient`

### SDK Error Taxonomy

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-foundation-sdk-errors`

The system **MUST** define `ResourceGroupError` enum in `resource-group-sdk/src/error.rs` covering all deterministic failure categories.

**Required variants**: `Validation`, `NotFound`, `TypeAlreadyExists`, `InvalidParentType`, `AllowedParentsViolation`, `CycleDetected`, `ConflictActiveReferences`, `LimitViolation`, `TenantIncompatibility`, `ServiceUnavailable`, `Internal`.

Each variant **MUST** carry structured context (field details for Validation, entity identifier for NotFound, conflicting code for TypeAlreadyExists, etc.) sufficient for the error mapping algorithm to produce informative Problem responses.

**Implements**:
- `cpt-cf-resource-group-algo-sdk-foundation-map-domain-error`

**Touches**:
- Entities: `ResourceGroupError`

### Persistence Adapter and DB Migrations

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-foundation-persistence`

The system **MUST** define SeaORM entity models and DB migration scripts for all 6 RG tables.

Each persistence adapter (type, group, closure, membership) **MUST** be defined as a trait first (e.g., `TypeRepositoryTrait`, `GroupRepositoryTrait`, `ClosureRepositoryTrait`, `MembershipRepositoryTrait`) and injected into domain services as `Arc<dyn Trait>`. This enables unit testing with in-memory trait implementations (`InMemoryTypeRepository`, etc.) without a database, and ensures a clean contract boundary between domain and infrastructure layers.

**Required tables** (per DESIGN 3.7):
- `gts_type` — SMALLINT PK (identity), `schema_id` (unique TEXT), `metadata_schema` (JSONB nullable), timestamps
- `gts_type_allowed_parent` — composite PK `(type_id, parent_type_id)` with CASCADE FK
- `gts_type_allowed_membership` — composite PK `(type_id, membership_type_id)` with CASCADE FK
- `resource_group` — UUID PK, `parent_id` FK (self-referential), `gts_type_id` FK, `name`, `metadata` (JSONB nullable), `tenant_id`, timestamps. Indexes: `(parent_id)`, `(name)`, `(gts_type_id, id)`, `(tenant_id)`
- `resource_group_membership` — unique `(group_id, gts_type_id, resource_id)`, FK group_id → resource_group, FK gts_type_id → gts_type, `created_at`. Index: `(gts_type_id, resource_id)`
- `resource_group_closure` — composite PK `(ancestor_id, descendant_id)`, `depth` INTEGER, FK both → resource_group. Indexes: `(descendant_id)`, `(ancestor_id, depth)`

**Constraints**: `cpt-cf-resource-group-constraint-db-agnostic`, `cpt-cf-resource-group-constraint-surrogate-ids-internal`

**Touches**:
- DB: `gts_type`, `gts_type_allowed_parent`, `gts_type_allowed_membership`, `resource_group`, `resource_group_membership`, `resource_group_closure`

### Module Scaffold and Initialization

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-foundation-module-scaffold`

The system **MUST** provide an RG module annotated with `#[modkit::module]` that registers SDK clients in ClientHub and establishes the phased initialization order for circular dependency resolution with AuthZ.

**Required behavior**:
- Phase 1 (SystemCapability): register `dyn ResourceGroupClient` and `dyn ResourceGroupReadHierarchy` in ClientHub. REST/gRPC endpoints NOT yet accepting traffic.
- Phase 2 (ready): start accepting REST/gRPC traffic. Write operations can now call `PolicyEnforcer` → `AuthZResolverClient` (available since AuthZ init in Phase 1).
- ClientHub registration: single `RgService` implementation registered as both `dyn ResourceGroupClient` and `dyn ResourceGroupReadHierarchy`.
- Query profile configuration loaded from module config (`max_depth`, `max_width`).

**Implements**:
- Sequence `cpt-cf-resource-group-seq-init-order`

**Constraints**: `cpt-cf-resource-group-constraint-no-authz-decision`

**Touches**:
- Entities: `RgModule`, `RgService`

### REST and OData Infrastructure

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-sdk-foundation-rest-odata`

The system **MUST** wire OperationBuilder-based REST API routing with OData `$filter` parsing and cursor-based pagination for all list endpoints.

**Required infrastructure**:
- OperationBuilder endpoint registration for types (under `/api/types-registry/v1/`), groups and memberships (under `/api/resource-group/v1/`)
- OData `$filter` parser supporting field-specific operators: `eq`, `ne`, `in` for string/UUID fields; `eq`, `ne`, `gt`, `ge`, `lt`, `le` for integer fields; nested path syntax (`hierarchy/parent_id`, `hierarchy/depth`)
- Cursor-based pagination: `limit` (1..200, default 25), `cursor` (opaque token). Ordering is undefined but consistent — no `$orderby` support.
- DomainError → Problem (RFC-9457) error response mapping wired into all endpoint handlers via OperationBuilder error hooks
- Path-based API versioning: `/api/resource-group/v1/` for groups and memberships, `/api/types-registry/v1/` for types

**Implements**:
- `cpt-cf-resource-group-algo-sdk-foundation-map-domain-error`

**Touches**:
- API: `GET/POST/PUT/DELETE /api/types-registry/v1/types*`, `GET/POST/PUT/DELETE /api/resource-group/v1/groups*`, `GET/POST/DELETE /api/resource-group/v1/memberships*`

## 6. Acceptance Criteria

- [x] SDK crate (`resource-group-sdk`) compiles with all model types, trait contracts, and error types defined
- [x] `GtsTypePath::new("gts.x.system.rg.type.v1~")` succeeds; `GtsTypePath::new("invalid")` returns validation error
- [x] All 6 DB tables are created by migration scripts with correct constraints and indexes
- [x] SeaORM entities compile and map to the DB schema without runtime errors
- [x] Module registers `dyn ResourceGroupClient` and `dyn ResourceGroupReadHierarchy` in ClientHub during Phase 1 init
- [x] All `ResourceGroupError` variants map to correct HTTP status codes and RFC-9457 Problem responses
- [x] OData `$filter` parser handles `eq`, `ne`, `in` operators and nested path syntax (`hierarchy/parent_id`)
- [x] Cursor-based pagination returns correct `PageInfo` with `has_next_page` and cursor tokens
- [x] No SMALLINT surrogate IDs appear in any SDK type, REST response schema, or trait method signature
- [x] Module does not contain any AuthZ decision logic, SQL filter generation, or policy evaluation code
