# PRD - Resource Group

## 1. Overview

### 1.1 Purpose

The Resource Group module provides a generic hierarchy and membership engine for organizing resources.

The module supports two usage profiles with one API surface:

- `catalog` profile: store and query arbitrary resource group structures and memberships.
- `ownership-graph` profile: expose deterministic hierarchy/membership reads that can be consumed by external decision systems (for example AuthZ plugin logic).

Cyber Fabric ships a ready-to-use Resource Group provider in `modules/system/resource-group`.
Deployments can either:

- use this built-in provider directly, or
- use a vendor-specific Resource Group provider behind the same read contracts (resolver/plugin pattern), analogous to Tenant Resolver extensibility.

For AuthZ-facing deployments, `ownership-graph` is the required profile. Provider strategy remains deployment-specific (built-in provider or vendor-specific provider).

Resource Group is data infrastructure only. It does not evaluate authorization policies and does not build SQL filters.

### 1.2 Background / Problem Statement

CyberFabric needs one consistent way to model hierarchical ownership and resource grouping. Without a shared module, each domain service re-implements tree logic, cycle prevention, traversal, and membership semantics.

Authorization flows additionally need a stable source for ownership hierarchy and group membership context. This source must be independent from policy logic and reusable outside AuthZ use cases.

### 1.3 Goals (Business Outcomes)

- Provide one stable contract for group type, entity, hierarchy, and membership operations.
- Enforce strict forest invariants (single parent, no cycles).
- Support dynamic type configuration through API and DB seeding.
- Provide efficient hierarchy operations using closure table.
- Allow AuthZ integration without coupling Resource Group to AuthZ semantics.

### 1.4 Non-goals

- Policy authoring or policy decisioning.
- SQL predicate generation for PEP query execution.
- Replacing AuthN/AuthZ resolver contracts.

### 1.5 Glossary

| Term | Definition |
|------|------------|
| Resource Group Type | Type schema for group entities and allowed parent type set. |
| Resource Group Entity | Concrete node in the hierarchy. |
| Membership | Explicit many-to-many link between group entity and resource identifier. |
| Forest | Collection of trees with single parent per node and no cycles. |
| Closure Table | Ancestor-descendant projection for efficient hierarchy queries. |
| Query Profile | Optional hierarchy guardrails `(max_depth, max_width)` used for performance/SLO tracking; limits can be disabled. |

## 2. Actors

### 2.1 Human Actors

#### Platform Operator

**ID**: `cpt-cf-resource-group-actor-platform-operator`

- **Role**: configures hierarchy query profile and operates migrations.
- **Needs**: predictable behavior when constraints are tightened.

#### Security Engineer

**ID**: `cpt-cf-resource-group-actor-security-engineer`

- **Role**: validates isolation and deterministic failure behavior in ownership graph usage.
- **Needs**: strict hierarchy invariants and fail-safe write validation.

### 2.2 System Actors

#### Domain Module Client

**ID**: `cpt-cf-resource-group-actor-domain-module-client`

- **Role**: manages types, groups, and memberships.

#### AuthZ Resolver Plugin (via AuthZ Resolver module)

**ID**: `cpt-cf-resource-group-actor-authz-plugin-consumer`

- **Role**: reads hierarchy/membership context from Resource Group to build AuthZ constraints.

## 3. Operational Concept & Environment

### 3.1 Core Boundary

Resource Group:

- owns hierarchy and membership data contracts.
- validates structural invariants and type compatibility.
- provides read models for consumers.

Resource Group does not:

- evaluate allow/deny decisions.
- interpret AuthZ policies.
- generate SQL or ORM filters.

### 3.2 AuthZ Integration Boundary (Fixed)

The integration point between AuthZ and Resource Group is at AuthZ plugin/PDP logic, not inside Resource Group.

- AuthZ plugin reads hierarchy/membership context from Resource Group.
- AuthZ plugin returns constraints in AuthZ response format.
- PEP (`PolicyEnforcer` + compiler) translates constraints to `AccessScope`/SQL.

This preserves approved AuthN/AuthZ architecture and keeps Resource Group AuthZ-agnostic.

### 3.3 Tenant Compatibility Rule for AuthZ Usage

When used in `ownership-graph` profile for AuthZ flows, groups are tenant-scoped:

- each group belongs to one tenant scope
- parent-child and membership links must satisfy tenant compatibility rules
- same-tenant links are always valid; cross-tenant links are valid only when tenants are related in configured tenant hierarchy scope
- AuthZ integration reads and downstream SQL compilation must be tenant-scoped by caller effective tenant scope (derived from `SecurityContext.subject_tenant_id` and tenant hierarchy visibility rules)

Operational exception for platform provisioning:

- privileged platform admin calls through `ResourceGroupClient` may run without caller tenant scoping when creating/managing tenant hierarchies across tenants
- this exception does not relax data invariants: every parent-child edge and membership link must still pass tenant hierarchy compatibility checks

This aligns Resource Group behavior with `docs/arch/authorization/RESOURCE_GROUP_MODEL.md`.

## 4. Scope

### 4.1 In Scope

- Dynamic type management API.
- Group entity lifecycle API.
- Closure-table-based hierarchy operations.
- Membership lifecycle and lookup operations.
- Query profile constraints (`max_depth`, `max_width`) and enforcement behavior.
- Generic read ports consumable by external modules/plugins.

### 4.2 Out of Scope

- AuthN/AuthZ resolver contract changes.
- PDP policy evaluation logic.
- SQL compilation engine changes in PEP.

## 5. Functional Requirements

### 5.1 Resource Group Type Management

#### Create, List, Get, Update, Delete Type

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-manage-types`

The module **MUST** provide API operations to create, list, retrieve, update, and delete resource group types.

A type includes:

- `code` (unique, case-insensitive)
- `parents` (allowed parent type codes)
- `owner_id`

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

#### Support Type Seeding

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-seed-types`

The module **MUST** support deterministic type seeding to initialize/update type definitions at startup/bootstrapping time.

#### Delete Type Only If Unused

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-delete-type-only-if-empty`

Type deletion **MUST** be rejected if at least one entity of that type exists.

### 5.2 Resource Group Entity Management

#### Create, Get, Update, Move, Delete Entity

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-manage-entities`

The module **MUST** provide API operations for:

- create entity
- retrieve entity by ID
- update mutable fields (`name`, `external_id`)
- move entity to new parent (subtree move)
- delete entity

Entity fields:

- `id` (UUIDv7)
- `type_code`
- `name` (1..255)
- `external_id` (optional, <=255)
- `parent_id` (optional)
- timestamps

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

#### Enforce Tenant Scope in Ownership-Graph Profile

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-tenant-scope-ownership-graph`

In `ownership-graph` profile, create/move/membership operations **MUST** reject tenant-incompatible links (including cross-tenant links outside configured tenant hierarchy scope).

### 5.3 Membership Management

#### Manage Membership Links

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-manage-membership`

The module **MUST** support add/remove membership links between group entity and resource identifier.

Membership persistence **MUST** store `tenant_id` for each link in `ownership-graph` profile.

#### Query Membership Relations

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-query-membership-relations`

The module **MUST** support deterministic membership lookups:

- by resource
- by group

### 5.4 Hierarchy Operations (Closure Table)

#### Use Closure Table Pattern

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-closure-table`

The system **MUST** provide efficient hierarchy queries using closure table.

Closure table **MUST** keep:

- `ancestor_id` (any ancestor on the path to `descendant_id`, at arbitrary depth)
- `descendant_id` (any descendant on the path from `ancestor_id`, at arbitrary depth)
- `depth` (0 for self)

Note: `parent_id/child_id` correspond specifically to the `depth == 1` case.
For authz-compatibility projections, `ancestor_id/descendant_id` are exported directly and `depth` is included as metadata so consumers can derive direct parent/child relationships (`depth == 1`) when needed.

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

### 5.6 AuthZ Integration Contract (Without Coupling)

#### Provide Generic Read Port for External Consumers

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-integration-read-port`

The module **MUST** expose stable read contracts for hierarchy/membership retrieval that external consumers (including AuthZ plugins) can use.

The same public read contract must remain stable across provider strategies:

- built-in Resource Group provider
- vendor-specific provider selected via resolver/plugin path

In `ownership-graph` profile, integration read responses **MUST** include `tenant_id` for each returned row:

- hierarchy reads (`resolve_descendants(ctx, ..)`, `resolve_ancestors(ctx, ..)`) return group row + tenant scope
- membership reads (`resolve_memberships(ctx, ..)`) return membership row + tenant scope
- integration read methods accept caller `SecurityContext`; Resource Group passes it through to selected provider path (for plugin path, pass-through is unchanged)
- in AuthZ query path, caller `SecurityContext.subject_tenant_id` is mandatory and used to resolve effective tenant scope for tenant-scoped reads and compiled SQL predicates
- when effective tenant scope contains multiple related tenants, read responses may contain rows with different `tenant_id` values

The read contract **MUST NOT** contain AuthZ decision semantics.

#### Keep Policy and SQL Semantics Outside Resource Group

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-no-authz-and-sql-logic`

Resource Group **MUST NOT**:

- return allow/deny policy decisions
- return AuthZ constraint objects
- return SQL fragments or ORM filters

### 5.7 Deterministic Error Semantics

- [ ] `p1` - **ID**: `cpt-cf-resource-group-fr-deterministic-errors`

The module **MUST** map all failures to deterministic categories:

- `validation`
- `not_found`
- `conflict` (`type already exists`, `invalid parent type`, `cycle`, `active references`)
- `limit_violation` (`max_depth`, `max_width`, when corresponding limit is enabled)
- `service_unavailable`
- `internal`

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

## 7. Public Library Interfaces

### 7.1 Public API Surface

#### Core Client Trait

- [ ] `p1` - **ID**: `cpt-cf-resource-group-interface-resource-group-client`

- **Type**: Rust trait API (`ResourceGroupClient`) via ClientHub
- **Description**: type/entity/membership lifecycle and hierarchy queries
- **Stability**: stable

#### Integration Read Trait

- [ ] `p1` - **ID**: `cpt-cf-resource-group-interface-integration-read-client`

- **Type**: Rust trait API (`ResourceGroupReadClient`) via ClientHub
- **Description**: read-only hierarchy/membership contract for external consumers (including AuthZ plugins)
- **Stability**: stable

Target trait shape (aligned with tenant-resolver pass-through `SecurityContext` pattern):

```rust
#[async_trait]
pub trait ResourceGroupReadClient: Send + Sync {
    async fn resolve_descendants(
        &self,
        ctx: &SecurityContext,
        root_id: Uuid,
    ) -> Result<Vec<ResourceGroupHierarchyRow>, ResourceGroupError>;

    async fn resolve_ancestors(
        &self,
        ctx: &SecurityContext,
        node_id: Uuid,
    ) -> Result<Vec<ResourceGroupHierarchyRow>, ResourceGroupError>;

    async fn resolve_memberships(
        &self,
        ctx: &SecurityContext,
        group_ids: Vec<Uuid>,
    ) -> Result<Vec<ResourceGroupMembershipRow>, ResourceGroupError>;
}
```

Companion plugin trait shape (gateway-internal delegation target):

```rust
#[async_trait]
pub trait ResourceGroupReadPluginClient: Send + Sync {
    async fn resolve_descendants(
        &self,
        ctx: &SecurityContext,
        root_id: Uuid,
    ) -> Result<Vec<ResourceGroupHierarchyRow>, ResourceGroupError>;

    async fn resolve_ancestors(
        &self,
        ctx: &SecurityContext,
        node_id: Uuid,
    ) -> Result<Vec<ResourceGroupHierarchyRow>, ResourceGroupError>;

    async fn resolve_memberships(
        &self,
        ctx: &SecurityContext,
        group_ids: Vec<Uuid>,
    ) -> Result<Vec<ResourceGroupMembershipRow>, ResourceGroupError>;
}
```

Gateway behavior:

- public callers use `ResourceGroupReadClient` from ClientHub
- module gateway resolves configured provider and either serves from built-in Resource Group data path or delegates to vendor-selected scoped plugin via `ResourceGroupReadPluginClient`
- `SecurityContext` is passed through unchanged when plugin path is selected

### 7.2 Integration Read Schemas (Ownership-Graph)

For AuthZ-facing usage, `ResourceGroupReadClient` returns data-only rows with explicit tenant scope.

Hierarchy read rows (`resolve_descendants(ctx, ..)`, `resolve_ancestors(ctx, ..)`):

| Field | Type | Required | Description |
|------|------|----------|-------------|
| `group_id` | UUID | Yes | Group identifier |
| `tenant_id` | UUID | Yes | Group tenant scope |
| `depth` | INT | Yes | Distance from input node (`0` = self) |

Membership read rows (`resolve_memberships(ctx, ..)`):

| Field | Type | Required | Description |
|------|------|----------|-------------|
| `group_id` | UUID | Yes | Group identifier |
| `tenant_id` | UUID | Yes | Membership tenant scope (stored on membership row; must match group tenant) |
| `resource_id` | UUID | Yes | Resource identifier |

Example calls:

```rust
let rg_read = hub.get::<dyn ResourceGroupReadClient>()?;

let authz_ctx = SecurityContext::builder()
    .subject_id(caller_subject_id)
    .subject_tenant_id(caller_tenant_id)
    .build()?;

let descendants = rg_read.resolve_descendants(&authz_ctx, root_group_id).await?;
let ancestors = rg_read.resolve_ancestors(&authz_ctx, group_id).await?;
let memberships = rg_read
    .resolve_memberships(&authz_ctx, vec![group_a, group_b])
    .await?;
```

These responses remain policy-agnostic and SQL-agnostic; caller-side PDP logic uses `tenant_id` to validate tenant ownership before producing group-based constraints.

## 8. Use Cases

### Scenario: Create Type

- **GIVEN** valid code `DEPARTMENT` and parents `["ORGANIZATION", "DIVISION"]`
- **WHEN** caller creates type
- **THEN** type is persisted with owner metadata

### Scenario: Reject Duplicate Type

- **GIVEN** type `DEPARTMENT` already exists
- **WHEN** caller creates same code
- **THEN** `TypeAlreadyExists`

### Scenario: Reject Invalid Type Code

- **GIVEN** code with whitespace or length > 63
- **WHEN** caller creates type
- **THEN** validation error

### Scenario: Create Entity with Parent

- **GIVEN** parent entity of type `ORGANIZATION`
- **AND** child type `DEPARTMENT` allows `ORGANIZATION`
- **WHEN** caller creates child with `parent_id`
- **THEN** entity and closure rows are created

### Scenario: Reject Invalid Parent Type

- **GIVEN** parent type not allowed by child type definition
- **WHEN** caller creates/moves entity
- **THEN** `InvalidParentType`

### Scenario: Move Subtree

- **GIVEN** entity with descendants and valid new parent
- **WHEN** caller moves subtree
- **THEN** closure rows are rebuilt for affected paths transactionally

### Scenario: Reject Cycle Creation

- **GIVEN** target parent is inside entity subtree
- **WHEN** caller attempts move
- **THEN** `CycleDetected`

### Scenario: Add Membership (Tenant-Compatible)

- **GIVEN** group `G1` and resource `R1` tenant scopes are compatible under configured tenant hierarchy rules
- **WHEN** caller invokes `add_membership` with caller `SecurityContext`
- **THEN** membership link `(G1, R1)` is created
- **AND** operation remains policy-agnostic (no AuthZ decision payload)

### Scenario: Reject Tenant-Incompatible Membership Add

- **GIVEN** group `G1` tenant scope is outside caller effective tenant scope (resolved from `subject_tenant_id`)
- **WHEN** tenant-scoped caller invokes `add_membership`
- **THEN** operation is rejected with deterministic validation/conflict category

### Scenario: Remove Membership

- **GIVEN** membership link `(G1, R1)` exists
- **WHEN** caller invokes `remove_membership` with caller `SecurityContext`
- **THEN** the link is removed
- **AND** tenant-incompatible attempts are rejected under ownership-graph tenant rules

### Scenario: Platform Admin Provisions Hierarchy Without Caller Tenant Scope

- **GIVEN** caller has privileged platform-admin capability for Resource Group provisioning
- **AND** caller request is not tenant-scoped by `subject_tenant_id`
- **WHEN** caller creates or moves tenant hierarchy nodes via `ResourceGroupClient`
- **THEN** operation is allowed for the explicit target tenant scope
- **AND** parent-child and membership links must still satisfy tenant hierarchy compatibility invariants

### Scenario: Reduced Query Profile Without Migration

- **GIVEN** stored tree exceeds newly tightened enabled limits
- **AND** no data migration was run
- **WHEN** read operation is executed
- **THEN** full stored data is returned
- **AND WHEN** violating write is attempted
- **THEN** write is rejected with `limit_violation`

### Scenario: AuthZ Consumer Reads Ownership Graph

- **GIVEN** AuthZ plugin needs hierarchy context
- **WHEN** plugin calls `ResourceGroupReadClient` with caller `SecurityContext`
- **THEN** Resource Group returns hierarchy/membership data only
- **AND** policy decision + constraint generation remain in AuthZ plugin
- **AND** SQL compilation remains in PEP layer

### Scenario: AuthZ Consumer Validates Tenant Scope from Read Rows

- **GIVEN** plugin calls `resolve_descendants` and `resolve_memberships` for candidate groups
- **WHEN** Resource Group returns rows with `tenant_id` for each entry
- **THEN** plugin validates each row `tenant_id` against caller effective tenant scope
- **AND** plugin excludes/rejects out-of-tenant groups before generating AuthZ constraints
- **AND** Resource Group still returns no policy decision fields

## 9. Acceptance Criteria

- [ ] Dynamic type API is available with validation and ownership metadata.
- [ ] Entity hierarchy remains strict forest under all operations.
- [ ] Closure-table ancestor/descendant queries are available and ordered by depth.
- [ ] Subtree move/delete are supported with transactional closure updates.
- [ ] Query profile (`max_depth`, `max_width`) behavior matches specified reduced-constraint rules, including disabled-limit (unlimited) mode.
- [ ] Resource Group remains AuthZ-agnostic while exposing integration read contracts.
- [ ] No changes are required in existing AuthN/AuthZ resolver contracts.
- [ ] Tenant-scoped constraints for AuthZ usage are enforced and tenant-incompatible links are rejected.
- [ ] Integration read rows include `tenant_id` in `ownership-graph` profile for deterministic caller-side tenant validation in AuthZ flows.
- [ ] `resource_group_membership` stores `tenant_id`, and AuthZ query path always uses effective tenant-scoped reads/SQL predicates.
- [ ] Platform-admin provisioning via Resource Group API may run without caller tenant scoping, while tenant hierarchy compatibility invariants remain enforced.

## 10. Dependencies

| Dependency | Description | Criticality |
|------------|-------------|-------------|
| SQL persistence layer | durable storage for types/entities/membership/closure | p1 |
| modkit/client_hub | typed inter-module client registration/discovery | p1 |
| AuthZ Resolver module | consumer of read contract via plugin path (optional consumer) | p1 |
| Vendor-specific RG provider (optional) | alternative backend behind same read contracts | p2 |

## 11. Assumptions

- AuthN/AuthZ module contracts remain unchanged and are extended only via plugins/adapters.
- Resource Group consumers depend on stable contracts (`ResourceGroupClient`, `ResourceGroupReadClient`), not on a specific provider implementation.
- Resource identifiers used in memberships are stable for consumer domain.
- Operators can run explicit migration scripts when tightening enabled query profile limits.

## 12. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Very deep/wide trees | degraded write performance on closure maintenance | depth/width validation, indexes, benchmark gates |
| Ambiguous ownership semantics between domains | inconsistent integration behavior | explicit type parent rules + integration contract tests |
| Misuse of Resource Group as policy engine | boundary drift and coupling | hard boundary in contracts, architecture review checks |

## 13. Open Questions

- Should delete behavior support both `leaf-only` and `subtree-cascade` modes in v1?
- Should non-UUID external resource IDs be first-class in membership schema, or remain adapter-mapped?

## 14. Traceability

- **Design**: [DESIGN.md](./DESIGN.md)
- **AuthN/AuthZ Architecture**: [docs/arch/authorization/DESIGN.md](../../../../docs/arch/authorization/DESIGN.md)
- **Resource Group Model**: [docs/arch/authorization/RESOURCE_GROUP_MODEL.md](../../../../docs/arch/authorization/RESOURCE_GROUP_MODEL.md)
