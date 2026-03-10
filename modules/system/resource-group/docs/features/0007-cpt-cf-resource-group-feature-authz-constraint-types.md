# Feature: AuthZ Advanced Constraint Types

- [x] `p1` - **ID**: `cpt-cf-resource-group-featstatus-authz-constraint-types`
- [x] `p1` - `cpt-cf-resource-group-feature-authz-constraint-types`

## 1. Feature Context

### 1.1 Overview

Extend the constraint model with `in_tenant_subtree`, `in_group`, and `in_group_subtree` predicate types as defined in the authorization architecture (`docs/arch/authorization/DESIGN.md`). Implement PEP constraint compiler support for these types, enabling SQL subquery generation through `tenant_closure`, `resource_group_closure`, and `resource_group_membership` local projection tables. Update the static-authz-plugin to return these advanced predicates when PEP capabilities declare support.

### 1.2 Purpose

Features 1-5 established the AuthZ enforcement pipeline with flat `In(OWNER_TENANT_ID, [tid])` predicates — single-level tenant isolation. The architecture mandates richer predicates for:
- **Tenant hierarchy traversal** — `in_tenant_subtree` enables access to child tenants' data without enumerating all descendant IDs
- **Group-based access control** — `in_group` / `in_group_subtree` enables resource filtering by group membership and group hierarchy via local closure tables
- **Capability negotiation** — PEP declares `capabilities: [TenantHierarchy, GroupMembership, GroupHierarchy]`, PDP returns matching predicate types

Without this feature, the PDP can only return flat `eq`/`in` predicates, requiring explicit ID enumeration. For tenants with deep hierarchies or large group trees, this is impractical.

Addresses:
- `docs/arch/authorization/DESIGN.md` — Predicate Types Reference (Section: `in_tenant_subtree`, `in_group`, `in_group_subtree`)
- `docs/arch/authorization/DESIGN.md` — Capabilities → Predicate Matrix
- `docs/arch/authorization/DESIGN.md` — Table Schemas (Local Projections)

### 1.3 Actors

| Actor | Role in Feature |
|-------|-----------------|
| Domain Module (PEP) | Declares capabilities in `EvaluationRequest`, receives advanced predicates, compiles to SQL |
| AuthZ Plugin (PDP) | Returns `in_tenant_subtree` / `in_group` / `in_group_subtree` predicates based on PEP capabilities |
| Static AuthZ Plugin | Extended to return `in_tenant_subtree` when `TenantHierarchy` capability declared |

### 1.4 References

- **Authorization Architecture**: `docs/arch/authorization/DESIGN.md` — Sections: Predicate Types Reference, Capabilities → Predicate Matrix, Table Schemas
- **RESOURCE_GROUP_MODEL.md**: `docs/arch/authorization/RESOURCE_GROUP_MODEL.md` — Closure table, membership table schemas
- **TENANT_MODEL.md**: `docs/arch/authorization/TENANT_MODEL.md` — Tenant hierarchy, barrier modes
- **AUTHZ_USAGE_SCENARIOS.md**: `docs/arch/authorization/AUTHZ_USAGE_SCENARIOS.md` — Concrete predicate examples
- **Dependencies**:
  - [x] `p1` - `cpt-cf-resource-group-feature-authz-enforcement` (Feature 5 — PolicyEnforcer pipeline)
  - [x] `p1` - `cpt-cf-resource-group-feature-entity-hierarchy` (Feature 3 — closure tables)
  - [x] `p2` - `cpt-cf-resource-group-feature-membership` (Feature 4 — membership table)

### 1.5 Current State

| Component | State | What exists |
|---|---|---|
| `constraints.rs` (SDK) | **DONE** | `Predicate::Eq`, `In`, `InTenantSubtree`, `InGroup`, `InGroupSubtree` |
| `compiler.rs` (SDK) | **DONE** | Compiles all 5 predicate types to `ScopeFilter` variants |
| `models.rs` (SDK) | `Capability` enum defined | `TenantHierarchy`, `GroupMembership`, `GroupHierarchy` |
| Static AuthZ Plugin | **DONE** | Returns `InTenantSubtree` when `TenantHierarchy` capability declared; flat `In` otherwise |
| PEP (PolicyEnforcer) | Capabilities propagated | `capabilities` via `with_capabilities()` in `EvaluationRequest` |
| SecureORM (modkit-db) | **DONE** | `ScopeFilter::InTenantSubtree`, `InGroup`, `InGroupSubtree` with SQL subquery generation |

## 2. Scope

### 2.1 SDK Changes (`authz-resolver-sdk`)

**New predicate types in `constraints.rs`:**

```rust
pub enum Predicate {
    Eq(EqPredicate),
    In(InPredicate),
    // New:
    InTenantSubtree(InTenantSubtreePredicate),
    InGroup(InGroupPredicate),
    InGroupSubtree(InGroupSubtreePredicate),
}
```

| Predicate | Fields | SQL Compilation |
|---|---|---|
| `InTenantSubtree` | `resource_property`, `root_tenant_id`, `barrier_mode`, `tenant_status` | `col IN (SELECT descendant_id FROM tenant_closure WHERE ancestor_id = ? AND barrier = 0)` |
| `InGroup` | `resource_property`, `group_ids` | `col IN (SELECT resource_id FROM resource_group_membership WHERE group_id IN (?))` |
| `InGroupSubtree` | `resource_property`, `root_group_id` | `col IN (SELECT resource_id FROM resource_group_membership WHERE group_id IN (SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = ?))` |

### 2.2 Compiler Changes (`pep/compiler.rs`)

Extend `compile_constraint()` to handle new predicate types. The compiler produces `ScopeFilter` variants that SecureORM translates to SQL.

### 2.3 SecureORM Changes (`modkit-db/src/secure/`)

Add new `ScopeFilter` variants for subquery-based filtering:

```rust
pub enum ScopeFilter {
    Eq(EqFilter),
    In(InFilter),
    // New:
    InTenantSubtree(InTenantSubtreeFilter),
    InGroup(InGroupFilter),
    InGroupSubtree(InGroupSubtreeFilter),
}
```

Each new variant generates the appropriate SQL subquery using SeaORM's `Condition::any()` / `Subselect`.

### 2.4 Static AuthZ Plugin Enhancement

When PEP declares `Capability::TenantHierarchy`:
- Return `InTenantSubtree` predicate instead of `In(OWNER_TENANT_ID, [tid])`
- Includes `barrier_mode` from request context

When PEP declares `Capability::GroupHierarchy` + group_id available:
- Additionally return `InGroupSubtree` or `InGroup` predicate

### 2.5 PEP Capability Declaration

RG module (and any other domain module PEPs) should declare appropriate capabilities in PolicyEnforcer calls based on which local projection tables they have.

## 3. Definitions of Done

### Predicate Type Extensions

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-predicate-types`

`authz-resolver-sdk/constraints.rs` **MUST** define `InTenantSubtree`, `InGroup`, `InGroupSubtree` predicate types with serde serialization matching the JSON format in `DESIGN.md`. Existing `Eq`/`In` predicates **MUST NOT** change (backward compatible).

### Constraint Compiler Extension

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-compiler-extension`

`authz-resolver-sdk/pep/compiler.rs` **MUST** compile new predicate types into `ScopeFilter` variants. Unknown predicate types **MUST** still fail-closed per existing behavior.

### SecureORM Subquery Filters

- [x] `p1` - **ID**: `cpt-cf-resource-group-dod-secure-orm-subquery`

`modkit-db/secure/` **MUST** support SQL generation for `InTenantSubtree`, `InGroup`, `InGroupSubtree` scope filters. SQL **MUST** use the local projection table schemas from `DESIGN.md` (Section: Table Schemas).

### Static Plugin Advanced Predicates

- [x] `p2` - **ID**: `cpt-cf-resource-group-dod-static-plugin-predicates`

Static AuthZ plugin **SHOULD** return `InTenantSubtree` when `TenantHierarchy` capability is declared, and `InGroupSubtree` when `GroupHierarchy` capability is declared with a group_id. Fallback to flat `In` when capabilities are absent.

## 4. Acceptance Criteria

- [x] `Predicate::InTenantSubtree` serializes/deserializes matching DESIGN.md JSON format
- [x] `Predicate::InGroup` serializes/deserializes matching DESIGN.md JSON format
- [x] `Predicate::InGroupSubtree` serializes/deserializes matching DESIGN.md JSON format
- [x] Compiler produces `ScopeFilter::InTenantSubtree` from `Predicate::InTenantSubtree`
- [x] Compiler produces `ScopeFilter::InGroup` from `Predicate::InGroup`
- [x] Compiler produces `ScopeFilter::InGroupSubtree` from `Predicate::InGroupSubtree`
- [x] SecureORM generates correct SQL subquery for `InTenantSubtree` using `tenant_closure`
- [x] SecureORM generates correct SQL subquery for `InGroup` using `resource_group_membership`
- [x] SecureORM generates correct SQL subquery for `InGroupSubtree` using both closure + membership
- [x] Barrier mode `respect` adds `AND barrier = 0` clause; `ignore` omits it
- [x] Static plugin returns `InTenantSubtree` when `TenantHierarchy` capability declared
- [x] Static plugin returns flat `In` when `TenantHierarchy` capability NOT declared (backward compatible)
- [x] Existing `Eq`/`In` predicate tests still pass (no regression)
- [x] PEP capability declaration propagated through `PolicyEnforcer` to `EvaluationRequest`

## 5. Test Plan

### Unit Tests — SDK (`constraints.rs`)
- Serialization roundtrip for all 5 predicate types
- Tag-based deserialization (`op` field) discriminates correctly
- `InTenantSubtree` with/without optional fields (barrier_mode, tenant_status)

### Unit Tests — Compiler (`compiler.rs`)
- `InTenantSubtree` compiles to `ScopeFilter::InTenantSubtree`
- `InGroup` compiles to `ScopeFilter::InGroup`
- `InGroupSubtree` compiles to `ScopeFilter::InGroupSubtree`
- Mixed old + new predicates in same constraint
- Unknown predicate type still fails closed
- Property validation still applies to new predicates

### Unit Tests — Static AuthZ Plugin
- Returns `InTenantSubtree` when `TenantHierarchy` capability present
- Returns flat `In` when `TenantHierarchy` capability absent
- Backward compatible with existing tests

### Integration Tests — SecureORM
- SQL generation for `InTenantSubtree` produces correct subquery
- SQL generation for `InGroup` produces correct subquery
- SQL generation for `InGroupSubtree` produces correct nested subquery
- Combined predicates (tenant + group) in same constraint

## 6. Design Notes

### InGroup / InGroupSubtree `resource_type` Filtering Semantics

The `InGroup` and `InGroupSubtree` predicates filter resources via the `resource_group_membership` table which has a composite key: `(group_id, resource_type, resource_id)`. The generated SQL subqueries select `resource_id` from memberships matching the group constraint.

**By design**, these predicates do **not** filter by `resource_type` in the SQL subquery:

```sql
-- InGroup SQL (current):
col IN (SELECT resource_id FROM resource_group_membership WHERE group_id IN (?))

-- InGroupSubtree SQL (current):
col IN (SELECT resource_id FROM resource_group_membership
        WHERE group_id IN (SELECT descendant_id FROM resource_group_closure WHERE ancestor_id = ?))
```

**Rationale**: The `resource_type` column in `resource_group_membership` is a domain classification (e.g., `"user"`, `"device"`, `"license"`), not a security boundary. The PEP's `ResourceType` descriptor determines which entity table the query targets — the `col` in the outer query is already the correct entity's primary key. Adding `resource_type` filtering to the subquery would require the PDP to know the entity classification, coupling policy to domain semantics.

**Consequence**: If the same `resource_id` value exists in multiple membership `resource_type`s within the same group, the subquery matches all of them. This is safe because:
1. `resource_id` values are UUIDs (globally unique) — collision across types is practically impossible
2. The outer query targets a specific entity table, providing implicit type filtering
3. The PEP's `ResourceType.supported_properties` already defines which entity is being queried

**Future consideration**: If non-UUID `resource_id` values are introduced (e.g., slugs that could collide across types), adding optional `resource_type` to `InGroup`/`InGroupSubtree` predicates with a `membership_resource_type` field should be considered. This would be a backward-compatible extension to the predicate schema.

### Tenant Closure Data Dependency

`InTenantSubtree` SQL reads from the `tenant_closure` local projection table. This table is created by migration `m20260310_000002_tenant_closure_projection` but is populated by the CDC pipeline (Feature 0008: `cpt-cf-resource-group-feature-tenant-closure-cdc`). Until the CDC pipeline is operational, `InTenantSubtree` queries will return empty results (no descendant tenants found), effectively behaving as `Eq(OWNER_TENANT_ID, root_tenant_id)` — only the root tenant itself is visible.

This is safe because:
- Empty `tenant_closure` means no subtree expansion — queries return a subset (not superset) of expected results
- The root tenant's self-row (if seeded) would still match
- No cross-tenant data leakage occurs from empty projection

See: `cpt-cf-resource-group-feature-tenant-closure-cdc` for the data population plan.

### ADR: Graceful Degradation for Group Hierarchy

When `static-authz-plugin` cannot resolve group hierarchy data (runtime call failure or client unavailable), it falls back to tenant-scoped constraints instead of denying access or returning an error. See [ADR-0001: Graceful Degradation](../ADR/0001-graceful-degradation-authz-plugin-hierarchy.md) for the full decision record.

## 7. Non-Applicable Domains

- **MTLS / Plugin Gateway**: Covered by Feature 6 (DEFERRED)
- **States (CDSL)**: Not applicable — constraint types are stateless evaluation artifacts
- **REST API changes**: None — constraints are internal PDP↔PEP contract
