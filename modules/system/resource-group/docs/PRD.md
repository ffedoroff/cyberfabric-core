Created:  2026-03-06 by Constructor Tech
Updated:  2026-03-06 by Constructor Tech

# PRD - Resource Group (RG)

> **Abbreviation**: Resource Group = **RG**. Used throughout this document.

## 1. Overview

### 1.1 Purpose

The RG module provides a generic hierarchy and membership engine for organizing resources.

**Concrete use-cases the module is designed for:**

| Domain | Use-case | How RG is used |
|--------|----------|----------------|
| **Organizational structure** | Model a multi-level institution: *University → Branch → Faculty → Department → Study Group*. | Each level is an RG Type; concrete units are RG Entities forming a tree. Admins manage sub-tenants, companies, regions, and divisions through a single tree view. |
| **Student Management (SMS)** | List students of a single study group, all students of a faculty, all students across branches of one institute. | `User` resources (students) are members of leaf-level groups. Hierarchy traversal returns flat or tree-shaped views at any depth — from a single group up to the entire institution. |
| **Learning Management (LMS)** | Organize courses and quizzes by department and program; assign the same course to multiple groups across faculties. | `Course` and `Quiz` resources are members of the relevant RG Entities. A course can appear in several groups within the same tenant, enabling per-program and per-branch listings. |
| **Content Management (CMS)** | Categorize learning materials (documents, videos, SCORM packages) into a multi-level content library and expose per-department or per-branch views. | `Content` resources are members of category-type RG Entities. Hierarchy queries produce both flat catalogues and nested navigation trees. |
| **Authorization context** | Derive "who can see what" from the organizational graph without hard-coding rules in each service. | AuthZ plugin reads the ownership graph produced by `ownership-graph` profile; RG itself makes no policy decisions. |

**Tenant isolation convention.** Every resource (`User`, `Course`, `Content`, etc.) belongs to exactly one tenant. RG treats `resource_id` as an opaque identifier and has no awareness of which tenant the referenced resource originates from. However, RG **does** enforce tenant consistency for memberships at the group level: when a resource already has memberships in groups belonging to tenant A, adding it to a group in an unrelated tenant B is rejected (see acceptance scenario "Reject Membership Add — Resource Already Linked in Incompatible Tenant"). This prevents a resource from spanning incompatible tenant scopes within RG, even though RG does not know the resource's "home" tenant. It is the caller's responsibility to ensure that the first membership correctly places the resource into a group of the resource's own tenant. Within its own tenant (or related tenants per hierarchy rules), a resource may be a member of multiple RG Entities simultaneously, enabling the platform to present the same data set in multiple organizational views depending on the business context.

A parent tenant may *read* resources of its child tenants for analytical and reporting purposes (e.g. a university admin views course statistics of a branch), but must not establish cross-tenant memberships. For example, an admin of the parent tenant can browse a course catalogue of a child tenant but cannot enroll in that course or assign it to a group in the parent tenant, because the admin is not a user of the child tenant and the course is not a resource of the parent tenant.

The module supports two usage profiles with one API surface:

- `catalog` profile: store and query arbitrary resource group structures and memberships.
- `ownership-graph` profile: expose deterministic hierarchy/membership reads that can be consumed by external decision systems (for example AuthZ plugin logic).

Cyber Fabric ships a ready-to-use RG provider in `modules/system/resource-group`.
Deployments can either:

- use this built-in provider directly, or
- use a vendor-specific RG provider behind the same read contracts (resolver/plugin pattern), analogous to Tenant Resolver extensibility.

For AuthZ-facing deployments, `ownership-graph` is the required profile. Provider strategy remains deployment-specific (built-in provider or vendor-specific provider).

RG is data infrastructure only. It does not evaluate authorization policies and does not build SQL filters.

### 1.2 Background / Problem Statement

CyberFabric needs one consistent way to model hierarchical ownership and resource grouping. Without a shared module, each domain service re-implements tree logic, cycle prevention, traversal, and membership semantics.

Authorization flows additionally need a stable source for ownership hierarchy and group membership context. This source must be independent from policy logic and reusable outside AuthZ use cases.

### 1.3 Goals (Business Outcomes)

- Provide one stable contract for group type, entity, hierarchy, and membership operations.
- Enforce strict forest invariants (single parent, no cycles).
- Support dynamic type configuration through API and DB seeding.
- Provide efficient hierarchy operations using closure table.
- Allow AuthZ integration without coupling RG to AuthZ semantics.

### 1.4 Non-goals

- Policy authoring or policy decisioning.
- SQL predicate generation for PEP query execution.
- Replacing AuthN/AuthZ resolver contracts.
- Being a mandatory dependency for AuthZ — AuthZ can operate without RG.

### 1.5 Glossary

| Term | Definition |
|------|------------|
| RG Type | Type schema for group entities and allowed parent type set. |
| RG Entity | Concrete node in the hierarchy (stored in `resource_group` table). |
| Resource Type | Caller-defined classification of a resource (e.g. `User`, `Document`). Part of membership composite key. |
| Membership | Explicit many-to-many link between group entity and resource identifier, qualified by resource type. Composite key: `(group_id, resource_type, resource_id)`. |
| Forest | Collection of trees with single parent per node and no cycles. |
| Closure Table | Ancestor-descendant projection for efficient hierarchy queries. |
| Query Profile | Optional hierarchy guardrails `(max_depth, max_width)` used for performance/SLO tracking; limits can be disabled. |

## 2. Actors

### 2.1 Human Actors

#### Instance Administrator

**ID**: `cpt-cf-resource-group-actor-instance-administrator`

- **Role**: manages resource group types, resource group items, and seeding (tenants). Configures hierarchy query profile and operates migrations.
- **Needs**: full control over type definitions, deterministic seeding, predictable behavior when constraints are tightened.

#### Tenant Administrator

**ID**: `cpt-cf-resource-group-actor-tenant-administrator`

- **Role**: within one tenant, manages sub items — groups, departments, sub-tenants.
- **Needs**: scoped management API, tenant-boundary enforcement, clear visibility of hierarchy within tenant scope.

### 2.2 System Actors

#### Apps

**ID**: `cpt-cf-resource-group-actor-apps`

- **Role**: programmatic access to RG via `ResourceGroupClient` SDK — manage types, groups, and memberships; read hierarchy and membership data.

#### AuthZ Resolver Plugin (via AuthZ Resolver module)

**ID**: `cpt-cf-resource-group-actor-authz-plugin-consumer`

- **Role**: reads hierarchy/membership context from RG to build AuthZ constraints. AuthZ can operate without RG — RG is an optional data source for AuthZ plugin logic.

## 3. Operational Concept & Environment

### 3.1 Core Boundary

RG:

- owns hierarchy and membership data contracts.
- validates structural invariants and type compatibility.
- provides read models for consumers.

RG does not:

- evaluate allow/deny decisions.
- interpret AuthZ policies.
- generate SQL or ORM filters.

### 3.2 AuthZ Integration Boundary (Fixed)

The integration point between AuthZ and RG is at AuthZ plugin/PDP logic, not inside RG.

- AuthZ plugin reads hierarchy/membership context from RG.
- AuthZ plugin returns constraints in AuthZ response format.
- PEP (`PolicyEnforcer` + compiler) translates constraints to `AccessScope`/SQL.

AuthZ can operate without RG. RG is an optional data source — AuthZ plugin logic decides whether to consume RG data. When RG is not deployed or not configured, AuthZ flows proceed without group-based constraints.

This preserves approved AuthN/AuthZ architecture and keeps RG AuthZ-agnostic.

### 3.3 Tenant Compatibility Rule for AuthZ Usage

When used in `ownership-graph` profile for AuthZ flows, groups are tenant-scoped:

- each group belongs to one tenant scope
- parent-child and membership links must satisfy tenant compatibility rules
- same-tenant links are always valid; cross-tenant links are valid only when tenants are related in configured tenant hierarchy scope
- AuthZ integration reads and downstream SQL compilation must be tenant-scoped by caller effective tenant scope (derived from `SecurityContext.subject_tenant_id` and tenant hierarchy visibility rules)

Operational exception for platform provisioning:

- privileged platform admin calls through `ResourceGroupClient` may run without caller tenant scoping when creating/managing tenant hierarchies across tenants
- this exception does not relax data invariants: every parent-child edge and membership link must still pass tenant hierarchy compatibility checks

This aligns RG behavior with `docs/arch/authorization/RESOURCE_GROUP_MODEL.md`.

## 4. Scope

### 4.1 In Scope

- Dynamic type management API.
- Group entity lifecycle API.
- Closure-table-based hierarchy operations.
- Membership lifecycle and lookup operations (qualified by `resource_type`).
- Query profile constraints (`max_depth`, `max_width`) and enforcement behavior.
- Generic read ports consumable by external modules/plugins.
- REST API endpoints (`/api/resource-group/v1/...`) with OData `$filter` and cursor-based pagination (`cursor`, `limit`).
- Deterministic type seeding for bootstrapping.

### 4.2 Out of Scope

- AuthN/AuthZ resolver contract changes.
- PDP policy evaluation logic.
- SQL compilation engine changes in PEP.

## 5. Functional Requirements

### 5.1 RG Type Management

#### Create, List, Get, Update, Delete Type

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-manage-types`

The module **MUST** provide API operations to create, list, retrieve, update, and delete resource group types.

A type includes:

- `code` (unique, case-insensitive)
- `can_be_root` (boolean; `true` means the type permits root placement — no `parent_id`)
- `allowed_parents` (allowed parent type codes; may be empty if the type is root-only). Invariant: `can_be_root OR len(allowed_parents) >= 1` — a type must have at least one valid placement

#### Validate Type Code Format

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-validate-type-code`

The module **MUST** validate type code format:

- length `1..63`
- no whitespace
- case-insensitive uniqueness

Invalid input **MUST** return validation error with field-specific details.

#### Reject Duplicate Type

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-reject-duplicate-type`

Creating a type with existing code **MUST** return `TypeAlreadyExists`.

#### Schema Migration and Type Data Seeding

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-seed-types`

Any RG plugin **MUST** perform schema migration (create/update required database schema) as part of its deployment lifecycle.

Type data seeding (populating type definitions) is **optional** and deployment-specific. It can be performed via:

- plugin data migration (built-in or custom)
- manual database administration
- RG API calls

AuthZ deployment determines which types are needed:

- **AuthZ does not use RG** — no type seeding required.
- **Flat tenants** — create type `tenant` with `can_be_root: true, allowed_parents: {}` (root placement only, no nesting).
- **Hierarchical tenants** — create type `tenant` with for example `can_be_root: true, allowed_parents: {'tenant'}` (root placement or nested under another tenant).

#### Delete Type Only If Unused

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-delete-type-only-if-empty`

Type deletion **MUST** be rejected if at least one entity of that type exists.

### 5.2 RG Entity Management

#### Create, Get, Update, Move, Delete Entity

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-manage-entities`

The module **MUST** provide API operations for:

- create entity
- retrieve entity by ID
- update mutable fields (`name`, `external_id`, `group_type`)
- move entity to new parent (subtree move)
- delete entity

Entity fields:

- `id` (UUID)
- `group_type` (reference to type code)
- `name` (1..255)
- `external_id` (optional, <=255)
- `parent_id` (optional)
- `tenant_id` (required)
- timestamps (`created`, `modified`) — stored in database for audit; not returned in REST API responses

In `ownership-graph` profile, entity also carries tenant scope metadata for tenant compatibility validation.

#### Enforce Forest Invariants

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-enforce-forest-hierarchy`

The hierarchy **MUST** remain a strict forest:

- single parent per entity
- no cycles

Cycle attempts **MUST** return `CycleDetected`.

#### Validate Parent Type Compatibility

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-validate-parent-type`

Entity create/move with parent **MUST** validate parent-child type compatibility against type definition.

Invalid relation **MUST** return `InvalidParentType`.

#### Delete Entity Only If No Active References

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-delete-entity-no-active-references`

Entity deletion **MUST** be rejected if active references/memberships prevent safe removal according to configured deletion policy.

#### Group Data Seeding

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-seed-groups`

Group data seeding (populating group hierarchy) is **optional** and deployment-specific. It can be performed via plugin data migration, manual database administration, or RG API calls. When performed, seeding **MUST** validate parent-child links and type compatibility. Repeated runs **MUST** be idempotent.

#### Enforce Tenant Scope in Ownership-Graph Profile

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-tenant-scope-ownership-graph`

In `ownership-graph` profile, create/move/membership operations **MUST** reject tenant-incompatible links (including cross-tenant links outside configured tenant hierarchy scope).

### 5.3 Membership Management

#### Manage Membership Links

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-manage-membership`

The module **MUST** support add/remove membership links between group entity and resource identifier, qualified by `resource_type`.

Membership composite key: `(group_id, resource_type, resource_id)`.

Membership fields:

- `group_id` (UUID, reference to group entity)
- `resource_type` (string, caller-defined resource classification)
- `resource_id` (string, caller-defined resource identifier)

Membership does not store `tenant_id` directly — tenant scope is derived from the referenced group's `tenant_id` via `group_id`. Membership requests are always scoped to a single tenant.

#### Query Membership Relations

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-query-membership-relations`

The module **MUST** support deterministic membership lookups:

- by resource (`resource_type` + `resource_id`)
- by group (`group_id`)
- by group and resource type (`group_id` + `resource_type`)

#### Membership Data Seeding

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-seed-memberships`

Membership data seeding (populating membership links) is **optional** and deployment-specific. It can be performed via plugin data migration, manual database administration, or RG API calls. When performed, seeding **MUST** validate group existence and tenant compatibility. Repeated runs **MUST** be idempotent.

### 5.4 Hierarchy Operations (Closure Table)

#### Use Closure Table Pattern

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-closure-table`

The system **MUST** provide efficient hierarchy queries using closure table.

Closure table **MUST** keep:

- `ancestor_id` (any ancestor on the path to `descendant_id`, at arbitrary depth)
- `descendant_id` (any descendant on the path from `ancestor_id`, at arbitrary depth)
- `depth` (0 for self)

Note: `parent_id` in the `resource_group` table corresponds to the `depth == 1` case (`ancestor_id` = `parent_id`, `descendant_id` = group itself).
For authz-compatibility projections, `ancestor_id/descendant_id` are exported directly and `depth` is included as metadata so consumers can derive direct parent relationships (`depth == 1`) when needed.

#### Ancestor and Descendant Queries

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-query-group-hierarchy`

The module **MUST** support:

- query all ancestors ordered by depth
- query all descendants ordered by depth

#### Efficient Subtree Move/Delete

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-subtree-operations`

The module **MUST** support efficient subtree move/delete operations with closure updates in transaction boundary.

### 5.5 Query Profile Constraints

#### Query Profile Configuration

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-query-profile`

Hierarchy query operations **MUST** apply service-level constraint configuration:

- `max_depth`:
  - optional positive integer
  - default `10` (recommended for fast default behavior)
  - configurable by deployment (including values `> 10`)
  - if disabled (`null`/absent): no depth limit
- `max_width`:
  - optional positive integer
  - if disabled (`null`/absent): no width limit

Effective `(max_depth, max_width)` **MUST** be treated as query profile for SLO tracking (including unlimited mode when limits are disabled).

#### Constraint Changes Must Not Rewrite Existing Data

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-profile-change-no-rewrite`

Changing query profile **MUST NOT** delete/rewrite existing hierarchy data.

#### Reduced Constraints Behavior

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-reduced-constraints-behavior`

If enabled limits are reduced and stored data exceeds new limits, and no migration has been run:

- read operations **MUST** return full stored data (no truncation by new limits)
- write operations that create/increase a violation **MUST** be rejected

Operator is responsible for separate data migration to restore compliance.

### 5.6 REST API and Query Support

#### REST API Endpoints

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-rest-api`

The module **MUST** expose REST API endpoints under `/api/resource-group/v1/` for:

- types: list, create, get, update, delete
- groups: list, create, get, update, delete
- memberships: list, add, remove

#### OData Query Support

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-odata-query`

List endpoints **MUST** support:

- `$filter` (OData v4.01) with field-specific operators (eq, ne, in)
- Cursor-based pagination per platform Cursor Pagination Spec (DNA/REST/QUERYING.md):
  - `limit` — page size (1..200, default 25)
  - `cursor` — opaque token from previous response for next/previous page
- Ordering is undefined but consistent — the server guarantees deterministic, stable order across pages within a pagination session, but does not commit to a specific sort order in the public contract. No `$orderby` support.

#### Group List with Hierarchy Depth

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-list-groups-depth`

A dedicated group depth endpoint (`/groups/{group_id}/hierarchy`) **MUST** return groups with a computed `depth` field (relative distance from reference group) and support depth-based filtering (`eq`, `ne`, `gt`, `ge`, `lt`, `le`). Positive depth = descendants, negative depth = ancestors, `0` = reference group itself.

#### Force Delete

- [ ] `p2` - **ID**: `cpt-cf-resource-group-fr-force-delete`

Group delete endpoint **MUST** support optional `force` query parameter to control cascade deletion behavior.

### 5.7 AuthZ Integration Contract (Without Coupling)

> Note: AuthZ can operate without RG. RG is an optional PIP data source for AuthZ plugin logic.

#### Provide Generic Read Port for External Consumers

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-integration-read-port`

The module **MUST** expose stable read contracts for hierarchy/membership retrieval that external consumers (including AuthZ plugins) can use.

The same public read contract must remain stable across provider strategies:

- built-in RG provider
- vendor-specific provider selected via resolver/plugin path

In `ownership-graph` profile, integration read responses match REST API schemas:

- hierarchy reads (`list_group_depth(ctx, group_id, query)`) return `Page<ResourceGroupWithDepth>` (matches REST `GET /groups/{group_id}/hierarchy`) — includes `tenant_id` per group
- membership reads (`list_memberships(ctx, query)`) return `Page<ResourceGroupMembership>` (matches REST `GET /memberships`) — no `tenant_id`; callers derive tenant scope from group data obtained via hierarchy reads
- integration read methods accept caller `SecurityContext`; RG passes it through to selected provider path (for plugin path, pass-through is unchanged)
- in AuthZ query path, caller `SecurityContext.subject_tenant_id` is mandatory and used to resolve effective tenant scope for tenant-scoped reads and compiled SQL predicates
- when effective tenant scope contains multiple related tenants, hierarchy read responses may contain rows with different `tenant_id` values

The read contract **MUST NOT** contain AuthZ decision semantics.

#### Keep Policy and SQL Semantics Outside RG

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-no-authz-and-sql-logic`

RG **MUST NOT**:

- return allow/deny policy decisions
- return AuthZ constraint objects
- return SQL fragments or ORM filters

### 5.8 Deterministic Error Semantics

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-deterministic-errors`

The module **MUST** map all failures to deterministic categories:

- `validation`
- `not_found`
- `conflict` (`type already exists`, `invalid parent type`, `cycle`, `active references`)
- `limit_violation` (`max_depth`, `max_width`, when corresponding limit is enabled)
- `service_unavailable`
- `internal`

### 5.9 Authentication Modes

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-dual-auth-modes`

RG REST API supports two authentication modes:

**JWT (public, all endpoints)**: standard user/service requests via bearer token. All endpoints available. Every request goes through AuthZ evaluation via `PolicyEnforcer` — identical flow to any other domain service (courses, users, etc.).

**MTLS (private, hierarchy endpoint only)**: service-to-service requests via mutual TLS client certificate. Used by AuthZ plugin to read tenant hierarchy. Only `GET /groups/{group_id}/hierarchy` is allowed — all other endpoints return `403 Forbidden`. MTLS requests **bypass AuthZ evaluation entirely** because:
- AuthZ plugin is the caller and cannot evaluate itself (circular dependency)
- MTLS certificate identity is a trusted system principal
- Single read-only endpoint — minimal attack surface

In monolith deployment, AuthZ uses `ResourceGroupReadHierarchy` via in-process ClientHub — no network, no MTLS, type system enforces hierarchy-only access. In microservice deployment, the same trait is backed by an MTLS-authenticated gRPC/REST call.

See DESIGN.md `cpt-cf-resource-group-seq-auth-modes` for detailed sequence diagrams.

## 6. Non-Functional Requirements

### 6.1 Hierarchy Query Latency

- [ ] `p1` - **ID**: `cpt-cf-resource-group-nfr-hierarchy-query-latency`

The module **MUST** support low-latency ancestor/descendant queries for depth up to configured query profile.

- **Threshold**: p95 under 50 ms for nominal default profile (`max_depth = 10`). For custom/unlimited profiles, target is deployment-specific and validated operationally.

### 6.2 Membership Query Latency

- [ ] `p1` - **ID**: `cpt-cf-resource-group-nfr-membership-query-latency`

The module **MUST** support low-latency membership reads.

- **Threshold**: p95 under 30 ms in nominal conditions.

### 6.3 Transactional Consistency

- [ ] `p1` - **ID**: `cpt-cf-resource-group-nfr-transactional-consistency`

Entity/membership changes and derived closure updates **MUST** be transactionally consistent.

### 6.4 Deterministic Error Coverage

- [ ] `p1` - **ID**: `cpt-cf-resource-group-nfr-deterministic-errors`

100% of failure paths **MUST** map to documented error categories.

### 6.5 Expected Production Scale

- [ ] `p1` - **ID**: `cpt-cf-resource-group-nfr-production-scale`

The module **MUST** be designed and validated for the following projected production volumes:

| Dimension | Projected Value |
|-----------|-----------------|
| Tenants (each a `resource_group` row) | ~1.5M |
| Total groups (tenants + org subgroups) | ~5M |
| Average hierarchy depth | ~3 |
| Closure rows (self-links + ancestry) | ~15.4M |
| Users (each with 1–2 memberships) | ~303.5M |
| Membership rows | ~455M |
| Projected total storage (data + indexes) | ~114 GB |

The membership table dominates storage at ~97% of total (~110 GB data + indexes). Index-to-data ratio is ~2.17× (reasonable for btree-only indexes with UUID keys; higher ratio reflects compact data rows relative to multi-column index entries).

Memory recommendations:

- Minimum: 24 GB RAM (shared_buffers = 6 GB)
- Recommended: 48 GB RAM (shared_buffers = 12 GB) to keep hot indexes in memory

Partitioning of `resource_group_membership` by tenant scope is a candidate optimization for production scale (see Open Questions).

### NFR Exclusions

- **Usability (UX)**: Not applicable — RG is a backend infrastructure module with no user-facing UI. Consumers interact via REST API and SDK traits.
- **Operations (OPS)**: Not applicable — RG follows standard CyberFabric deployment and monitoring patterns. No module-specific operational requirements beyond platform defaults.
- **Compliance (COMPL)**: Not applicable — RG does not directly handle PII or regulated data. Compliance requirements are owned by consuming modules and platform-level controls.
- **Safety (SAFE)**: Not applicable — RG is a data infrastructure module with no physical interaction or safety-critical operations.

## 7. Public Library Interfaces

### 7.1 Public API Surface

#### Core Client Trait

- [ ] `p1` - **ID**: `cpt-cf-resource-group-interface-resource-group-client`

- **Type**: Rust trait API (`ResourceGroupClient`) via ClientHub
- **Description**: type/entity/membership lifecycle and hierarchy queries
- **Stability**: stable

#### Integration Read Traits

- [ ] `p1` - **ID**: `cpt-cf-resource-group-interface-integration-read-hierarchy`

- **Type**: Rust trait API (`ResourceGroupReadHierarchy`) via ClientHub
- **Description**: narrow hierarchy-only read contract for AuthZ plugins
- **Stability**: stable

Integration read models reuse REST-aligned SDK structs (see DESIGN.md for full definitions):

- `list_group_depth` returns `Page<ResourceGroupWithDepth>` (matches REST `GET /groups/{group_id}/hierarchy`)
- `list_memberships` returns `Page<ResourceGroupMembership>` (matches REST `GET /memberships` — no `tenant_id`; tenant scope derived from group data via hierarchy reads)

Target trait shape:

```rust
/// Narrow hierarchy-only read contract. Used by AuthZ plugin.
#[async_trait]
pub trait ResourceGroupReadHierarchy: Send + Sync {
    /// Matches REST `GET /groups/{group_id}/hierarchy` with OData query.
    async fn list_group_depth(
        &self,
        ctx: &SecurityContext,
        group_id: Uuid,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupWithDepth>, ResourceGroupError>;
}
```

General consumers use `ResourceGroupClient` for both read and write operations (including `list_group_depth` and `list_memberships`).

Companion plugin trait shape (gateway-internal delegation target):

```rust
/// Plugin read contract. Extends `ResourceGroupReadHierarchy`.
#[async_trait]
pub trait ResourceGroupReadPluginClient: ResourceGroupReadHierarchy {
    async fn list_memberships(
        &self,
        ctx: &SecurityContext,
        query: ListQuery,
    ) -> Result<Page<ResourceGroupMembership>, ResourceGroupError>;
}
```

Gateway behavior:

- AuthZ plugin uses `ResourceGroupReadHierarchy` from ClientHub (hierarchy only)
- general consumers use `ResourceGroupClient` from ClientHub (full CRUD including reads)
- both backed by the same implementation, registered twice in ClientHub
- module gateway resolves configured provider and either serves from built-in RG data path or delegates to vendor-selected scoped plugin via `ResourceGroupReadPluginClient`
- `SecurityContext` is passed through unchanged when plugin path is selected

### 7.2 Authentication Modes

See `cpt-cf-resource-group-fr-dual-auth-modes` in section 5.9 for the full authentication modes requirement.

### 7.3 Integration Read Schemas (Ownership-Graph)

For AuthZ-facing usage, `ResourceGroupReadHierarchy` returns data-only rows. Schemas match REST API models.

Hierarchy read rows (`list_group_depth(ctx, group_id, query)`) — `Page<ResourceGroupWithDepth>` (matches REST `GET /groups/{group_id}/hierarchy`):

| Field | Type | Required | Description |
|------|------|----------|-------------|
| `group_id` | UUID | Yes | Group identifier |
| `parent_id` | UUID / null | No | Parent group (null for root) |
| `group_type` | string | Yes | Type code |
| `name` | string | Yes | Display name |
| `tenant_id` | UUID | Yes (ownership-graph) | Group tenant scope |
| `external_id` | string / null | No | Optional external ID |
| `depth` | INT | Yes | Relative distance from input node (`0` = self) |

Membership read rows (`list_memberships(ctx, query)`) — `Page<ResourceGroupMembership>` (matches REST `GET /memberships`):

| Field | Type | Required | Description |
|------|------|----------|-------------|
| `group_id` | UUID | Yes | Group identifier |
| `resource_type` | string | Yes | Resource type classification |
| `resource_id` | string | Yes | Resource identifier |

Membership rows do not include `tenant_id`. Callers derive tenant scope from group data obtained via `list_group_depth`.

Example calls:

```rust
// AuthZ plugin — uses narrow ResourceGroupReadHierarchy (hierarchy only)
let rg_hierarchy = hub.get::<dyn ResourceGroupReadHierarchy>()?;

let authz_ctx = SecurityContext::builder()
    .subject_id(caller_subject_id)
    .subject_tenant_id(caller_tenant_id)
    .build()?;

// descendants of root_group_id
let descendants = rg_hierarchy
    .list_group_depth(&authz_ctx, root_group_id, ListQuery::new().filter("depth ge 0"))
    .await?;

// ancestors of group_id
let ancestors = rg_hierarchy
    .list_group_depth(&authz_ctx, group_id, ListQuery::new().filter("depth le 0"))
    .await?;

// General consumer — uses ResourceGroupClient (full CRUD including reads)
let rg = hub.get::<dyn ResourceGroupClient>()?;

// memberships for specific groups
let memberships = rg
    .list_memberships(&authz_ctx, ListQuery::new().filter("group_id in ('...',  '...')"))
    .await?;
```

These responses remain policy-agnostic and SQL-agnostic; caller-side PDP logic uses `tenant_id` from hierarchy rows to validate tenant ownership before producing group-based constraints.

## 8. Use Cases

### 8.1 Types

#### Scenario: Create Type

- **GIVEN** valid code `branch` and allowed_parents `["tenant", "department"]`
- **WHEN** caller creates type
- **THEN** type is persisted with owner metadata

#### Scenario: Reject Duplicate Type

- **GIVEN** type `branch` already exists
- **WHEN** caller creates same code
- **THEN** `TypeAlreadyExists`

#### Scenario: Reject Invalid Type Code

- **GIVEN** code with whitespace or length > 63
- **WHEN** caller creates type
- **THEN** validation error

#### Scenario: Update Type Parents (Compatible)

- **GIVEN** type `branch` exists with allowed_parents `["tenant"]`
- **AND** no existing `branch` groups have a parent of type `department`
- **WHEN** caller updates allowed_parents to `["tenant", "department"]` (adds new allowed parent)
- **THEN** type definition is updated
- **AND** existing groups remain valid

#### Scenario: Reject Update Type Parents When Existing Groups Violate New Rules

- **GIVEN** type `branch` exists with allowed_parents `["tenant", "department"]`
- **AND** group `B1` (type `branch`) has parent `D1` (type `department`)
- **WHEN** caller updates `branch` allowed_parents to `["tenant"]` (removes `department`)
- **THEN** update is rejected with `conflict` — existing group `B1` would violate new parent type rules

#### Scenario: Delete Unused Type

- **GIVEN** type `team` exists
- **AND** no groups of type `team` exist in the database
- **WHEN** caller deletes type `team`
- **THEN** type is removed

#### Scenario: Reject Delete Type With Existing Groups

- **GIVEN** type `branch` exists
- **AND** at least one group of type `branch` exists
- **WHEN** caller deletes type `branch`
- **THEN** deletion is rejected with `conflict` — type is in use

#### Scenario: Seed Types (Pre-Deployment Step)

- **GIVEN** deployment configuration defines types `["tenant", "department", "branch"]`
- **WHEN** operator runs the seeding step before deployment (pre-deployment migration)
- **THEN** missing types are created, existing types are updated to match seed definitions
- **AND** seeding is idempotent — repeated runs produce the same result

### 8.2 Groups

#### Scenario: Create Root Entity

- **GIVEN** type `tenant` exists with `can_be_root: true` (permits root placement)
- **WHEN** caller creates group with type `tenant`, name `"Acme Corp"`, no `parent_id`
- **THEN** root entity is created with self-referencing closure row (depth 0)

#### Scenario: Create Entity with Parent

- **GIVEN** parent entity `D1` of type `department`
- **AND** child type `branch` allows `department` as parent
- **WHEN** caller creates child with `parent_id = D1`
- **THEN** entity and closure rows are created (self at depth 0, parent at depth 1, transitive ancestors at depth N)

#### Scenario: Reject Create Entity with Nonexistent Parent

- **GIVEN** `parent_id` references a UUID that does not exist in `resource_group` table
- **WHEN** caller creates entity with that `parent_id`
- **THEN** `not_found` — parent group does not exist

#### Scenario: Reject Invalid Parent Type

- **GIVEN** type `team` allows allowed_parents `["branch"]` only
- **AND** parent entity `D1` has type `department`
- **WHEN** caller creates group of type `team` with `parent_id = D1`
- **THEN** `InvalidParentType` — `department` is not in allowed_parents for `team`

#### Scenario: Move Subtree to Valid Parent

- **GIVEN** group `B1` (type `branch`) with descendants `[T1, T2]`
- **AND** new parent `D2` (type `department`) is in a different subtree, same tenant
- **AND** `branch` allows `department` as parent
- **WHEN** caller moves `B1` to `parent_id = D2`
- **THEN** closure rows are rebuilt for `B1`, `T1`, `T2` transactionally
- **AND** all ancestor paths now go through `D2`

#### Scenario: Reject Move — Cycle Detection

- **GIVEN** hierarchy: `D1 → B1 → T1`
- **WHEN** caller attempts to move `D1` under `T1` (`parent_id = T1`)
- **THEN** `CycleDetected` — `T1` is a descendant of `D1`, so making `T1` the parent of `D1` creates a cycle

#### Scenario: Reject Move — Self-Parent

- **GIVEN** group `B1` exists
- **WHEN** caller attempts to move `B1` under itself (`parent_id = B1`)
- **THEN** `CycleDetected` — a node cannot be its own parent

#### Scenario: Reject Move — Incompatible Parent Type at New Location

- **GIVEN** group `B1` (type `branch`) currently under `D1` (type `department`)
- **AND** new target parent `T1` has type `team`
- **AND** `branch` does not allow `team` as parent
- **WHEN** caller moves `B1` to `parent_id = T1`
- **THEN** `InvalidParentType`

#### Scenario: Reject Move — Tenant Scope Change via Parent

- **GIVEN** group `B1` (type `branch`, `tenant_id = A`) currently under `D1` (`tenant_id = A`)
- **AND** new target parent `D2` belongs to `tenant_id = B`
- **WHEN** caller moves `B1` to `parent_id = D2`
- **THEN** operation is rejected — changing `parent_id` to a node in a different tenant would implicitly change tenant ownership of `B1` and its descendants, which is prohibited via API

#### Scenario: Reject Create Entity — Cross-Tenant Parent (Unrelated Tenants)

- **GIVEN** parent entity `D1` belongs to tenant `A`
- **AND** caller creates child entity in tenant `B`
- **AND** tenants `A` and `B` are not related in configured tenant hierarchy
- **WHEN** caller creates entity with `parent_id = D1`
- **THEN** operation is rejected — tenant-incompatible parent-child link

#### Scenario: Update Entity Mutable Fields

- **GIVEN** group `B1` exists with name `"Engineering"` and no `external_id`
- **WHEN** caller updates `B1` with name `"Platform Engineering"` and `external_id = "ENG-001"`
- **THEN** mutable fields are updated, `id`, `group_type`, `parent_id`, `tenant_id` remain unchanged

#### Scenario: Delete Leaf Entity

- **GIVEN** group `T1` has no children and no active memberships
- **WHEN** caller deletes `T1`
- **THEN** entity and its closure rows are removed

#### Scenario: Reject Delete Entity With Children

- **GIVEN** group `B1` has children `[T1, T2]`
- **AND** `force` parameter is not set
- **WHEN** caller deletes `B1`
- **THEN** deletion is rejected — active child references exist

#### Scenario: Reject Delete Entity With Active Memberships

- **GIVEN** group `T1` has no children but has membership links `[(T1, User, R1), (T1, User, R2)]`
- **AND** `force` parameter is not set
- **WHEN** caller deletes `T1`
- **THEN** deletion is rejected — active membership references exist

#### Scenario: Reject Create Entity — Max Depth Exceeded

- **GIVEN** query profile `max_depth = 3`
- **AND** hierarchy already has depth 3: `L0 → L1 → L2 → L3`
- **WHEN** caller creates entity with `parent_id = L3` (would produce depth 4)
- **THEN** `limit_violation` — `max_depth` exceeded

#### Scenario: List Groups — Tenant Admin Sees Only Tenant-Scoped Subtree

- **GIVEN** stored hierarchy:
  ```
  T1 (tenant, tenant_id=T1)
  ├── D1 (department, tenant_id=T1)
  └── T7 (tenant, tenant_id=T7)
      ├── D2 (department, tenant_id=T7)
      └── D3 (department, tenant_id=T7)
  T9 (tenant, tenant_id=T9)
  └── D4 (department, tenant_id=T9)
  ```
- **AND** Tenant Administrator authenticates with `SecurityContext.subject_tenant_id = T7`
- **WHEN** Tenant Admin calls `GET /api/resource-group/v1/groups` without any `$filter`
- **THEN** AuthZ evaluates the request via `PolicyEnforcer` and produces `AccessScope {tenant_id IN (T7)}`
- **AND** SecureORM appends `WHERE tenant_id IN ('T7')` to the query
- **AND** response contains only groups visible to tenant `T7`: `[T7, D2, D3]`
- **AND** groups from other tenants (`T1`, `D1`, `T9`, `D4`) are not returned

#### Scenario: Seed Groups (Pre-Deployment Step)

- **GIVEN** deployment configuration defines group hierarchy:
  ```
  T1 (type=tenant, name="Root Tenant")
  ├── T3 (type=tenant, name="Child Tenant A", parent=T1)
  └── T7 (type=tenant, name="Child Tenant B", parent=T1)
  ```
- **WHEN** operator runs the seeding step before deployment (pre-deployment migration)
- **THEN** missing groups are created with closure rows, existing groups are updated to match seed definitions
- **AND** parent-child links and type compatibility are validated during seeding
- **AND** seeding is idempotent — repeated runs produce the same result

#### Scenario: Reduced Query Profile Without Migration

- **GIVEN** stored tree exceeds newly tightened enabled limits
- **AND** no data migration was run
- **WHEN** read operation is executed
- **THEN** full stored data is returned
- **AND WHEN** violating write is attempted
- **THEN** write is rejected with `limit_violation`

### 8.3 Membership

#### Scenario: Add Membership (Tenant-Compatible)

- **GIVEN** group `G1` (tenant `A`) and resource `(User, R1)`
- **AND** caller `SecurityContext.subject_tenant_id` is compatible with tenant `A`
- **WHEN** caller invokes `add_membership` with `group_id = G1`, `resource_type = "User"`, `resource_id = "R1"`
- **THEN** membership link `(G1, User, R1)` is created
- **AND** operation remains policy-agnostic (no AuthZ decision payload)

#### Scenario: Add Membership — Multiple Resource Types in Same Group

- **GIVEN** group `G1` exists
- **WHEN** caller adds `(G1, User, U1)` then `(G1, Document, DOC1)`
- **THEN** both membership links are created — a group can have members of different resource types

#### Scenario: Reject Duplicate Membership

- **GIVEN** membership link `(G1, User, R1)` already exists
- **WHEN** caller invokes `add_membership` with same `(G1, User, R1)`
- **THEN** operation is rejected with `conflict` — membership already exists

#### Scenario: Reject Membership Add — Group Does Not Exist

- **GIVEN** `group_id` references a UUID that does not exist
- **WHEN** caller invokes `add_membership`
- **THEN** `not_found` — target group does not exist

#### Scenario: Reject Tenant-Incompatible Membership Add

- **GIVEN** group `G1` belongs to tenant `A`
- **AND** caller `SecurityContext.subject_tenant_id` resolves to tenant `B`
- **AND** tenants `A` and `B` are not related in configured tenant hierarchy
- **WHEN** caller invokes `add_membership` for group `G1`
- **THEN** operation is rejected with deterministic validation/conflict category

#### Scenario: Reject Membership Add — Resource Already Linked in Incompatible Tenant

- **GIVEN** resource `(User, R1)` has existing membership in group `G1` (tenant `A`)
- **AND** caller attempts to add `(User, R1)` to group `G2` (tenant `B`)
- **AND** tenants `A` and `B` are not related in configured tenant hierarchy
- **WHEN** caller invokes `add_membership` for `(G2, User, R1)`
- **THEN** operation is rejected — resource membership would span incompatible tenant scopes

#### Scenario: Remove Membership

- **GIVEN** membership link `(G1, User, R1)` exists
- **WHEN** caller invokes `remove_membership` with `group_id = G1`, `resource_type = "User"`, `resource_id = "R1"`
- **THEN** the link is removed

#### Scenario: Remove Nonexistent Membership

- **GIVEN** no membership link `(G1, User, R99)` exists
- **WHEN** caller invokes `remove_membership` for `(G1, User, R99)`
- **THEN** `not_found` — membership does not exist

#### Scenario: Query Memberships by Group

- **GIVEN** group `G1` has memberships `[(G1, User, U1), (G1, User, U2), (G1, Document, D1)]`
- **WHEN** caller queries memberships with `$filter=group_id eq 'G1'`
- **THEN** all three membership links are returned

#### Scenario: Query Memberships by Resource

- **GIVEN** resource `(User, U1)` is a member of groups `G1`, `G2`, `G3`
- **WHEN** caller queries memberships with `$filter=resource_type eq 'User' and resource_id eq 'U1'`
- **THEN** three membership links are returned: `(G1, User, U1)`, `(G2, User, U1)`, `(G3, User, U1)`

#### Scenario: Seed Memberships (Pre-Deployment Step)

- **GIVEN** deployment configuration defines membership links:
  - `(G1, User, admin-user-1)`
  - `(G1, ServiceAccount, svc-monitoring)`
  - `(G2, User, admin-user-1)`
- **WHEN** operator runs the seeding step before deployment (pre-deployment migration)
- **THEN** missing membership links are created, existing links are preserved
- **AND** group existence and tenant compatibility are validated during seeding
- **AND** seeding is idempotent — repeated runs produce the same result

### 8.4 Group Hierarchy (MTLS AuthZ)

#### Scenario: AuthZ Plugin Resolves Tenant Hierarchy Downward

- **GIVEN** stored hierarchy (all groups of type `tenant` or `group`):
  ```
  T1 (tenant, tenant_id=T1)
  ├── T3 (tenant, tenant_id=T3)
  │   └── G10 (group, tenant_id=T3)
  └── T7 (tenant, tenant_id=T7)
      ├── G20 (group, tenant_id=T7)
      └── G21 (group, tenant_id=T7)
  ```
- **AND** AuthZ plugin needs to resolve which tenants/groups are visible to a user whose `subject_tenant_id = T1`
- **WHEN** plugin calls `list_group_depth(ctx, T1, filter="depth ge 0 and group_type in ('tenant','group')")` via MTLS
- **THEN** RG returns:
  | group_id | group_type | tenant_id | depth |
  |----------|------------|-----------|-------|
  | T1       | tenant     | T1        | 0     |
  | T3       | tenant     | T3        | 1     |
  | T7       | tenant     | T7        | 1     |
  | G10      | group      | T3        | 2     |
  | G20      | group      | T7        | 2     |
  | G21      | group      | T7        | 2     |
- **AND** plugin uses `tenant_id` from each row to build tenant-scoped AuthZ constraints
- **AND** RG returns no policy decisions — only data rows

#### Scenario: MTLS Request to Non-Hierarchy Endpoint

- **GIVEN** caller authenticates via MTLS client certificate
- **WHEN** caller sends request to `POST /api/resource-group/v1/groups` (non-hierarchy endpoint)
- **THEN** `403 Forbidden` — MTLS mode only allows `GET /groups/{group_id}/hierarchy`

## 9. Acceptance Criteria

- [ ] Dynamic type API is available with validation.
- [ ] Entity hierarchy remains strict forest under all operations.
- [ ] Closure-table ancestor/descendant queries are available and ordered by depth.
- [ ] Subtree move/delete are supported with transactional closure updates.
- [ ] Query profile (`max_depth`, `max_width`) behavior matches specified reduced-constraint rules, including disabled-limit (unlimited) mode.
- [ ] RG remains AuthZ-agnostic while exposing integration read contracts.
- [ ] No changes are required in existing AuthN/AuthZ resolver contracts.
- [ ] Tenant-scoped constraints for AuthZ usage are enforced and tenant-incompatible links are rejected.
- [ ] Integration read hierarchy rows include `tenant_id` (via `ResourceGroupWithDepth`); membership rows match REST `Membership` schema (no `tenant_id`). Callers derive membership tenant scope from group data.
- [ ] `resource_group_membership` derives tenant scope from the referenced group's `tenant_id` via `group_id` JOIN, and AuthZ query path always uses effective tenant-scoped reads/SQL predicates.
- [ ] Platform-admin provisioning via RG API may run without caller tenant scoping, while tenant hierarchy compatibility invariants remain enforced.
- [ ] Membership operations use composite key `(group_id, resource_type, resource_id)`.
- [ ] REST API endpoints available under `/api/resource-group/v1/` with OData query support.
- [ ] Dedicated group depth endpoint returns relative `depth` and supports depth-based filtering.

## 10. Dependencies

| Dependency | Description | Criticality |
|------------|-------------|-------------|
| SQL persistence layer (database-agnostic) | durable storage for types/entities/membership/closure; no vendor-specific SQL extensions | p1 |
| modkit/client_hub | typed inter-module client registration/discovery | p1 |
| AuthZ Resolver module | consumer of read contract via plugin path (optional consumer) | p1 |
| Vendor-specific RG provider (optional) | alternative backend behind same read contracts | p2 |

## 11. Assumptions

- AuthN/AuthZ module contracts remain unchanged and are extended only via plugins/adapters.
- RG consumers depend on stable contracts (`ResourceGroupClient`, `ResourceGroupReadHierarchy`), not on a specific provider implementation.
- Resource identifiers used in memberships are stable for consumer domain.
- Operators can run explicit migration scripts when tightening enabled query profile limits.

## 12. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Very deep/wide trees | degraded write performance on closure maintenance | depth/width validation, indexes, benchmark gates |
| Ambiguous ownership semantics between domains | inconsistent integration behavior | explicit type parent rules + integration contract tests |
| Misuse of RG as policy engine | boundary drift and coupling | hard boundary in contracts, architecture review checks |

## 13. Open Questions

- Should delete behavior support both `leaf-only` and `subtree-cascade` modes in v1? (REST API defines `force` query parameter for cascade control.) **Owner**: platform team. **Target**: resolve before DECOMPOSITION.
- Should `resource_group_membership` be partitioned for production scale (projected 455M rows, ~110 GB)? Partitioning strategy needs evaluation since tenant scope is derived via group FK, not stored directly. **Owner**: platform team. **Target**: resolve before production deployment.
- Should RG type `code` and membership `resource_type` be validated against GTS (Global Type System)? Current design: both are free-form strings with no external validation. Proposed alternative: validate that values correspond to registered GTS types at write time (type create/update for `code`, membership add for `resource_type`), keeping all other behavior unchanged (local storage, no runtime GTS dependency for reads). The case is stronger for `resource_type` — it references external domain entities (e.g., "User", "Document") that likely already exist as GTS types, and without validation nothing prevents typos or inconsistent naming. For `code`, the case is weaker — RG defines its own type topology, not referencing external concepts. Trade-offs: GTS validation adds governance and cross-module type consistency, but introduces seed ordering dependency (types-registry must be available before RG writes), and adds a soft dependency on types-registry availability for write operations. Current recommendation: defer until cross-module type reuse creates an actual governance need; `resource_type` validation is a stronger candidate to adopt first. **Owner**: platform team. **Target**: revisit when other modules begin referencing the same type codes or resource types.

## 14. Traceability

- **Design**: [DESIGN.md](./DESIGN.md)
- **AuthN/AuthZ Architecture**: [docs/arch/authorization/DESIGN.md](../../../../docs/arch/authorization/DESIGN.md)
- **RG Model**: [docs/arch/authorization/RESOURCE_GROUP_MODEL.md](../../../../docs/arch/authorization/RESOURCE_GROUP_MODEL.md)
